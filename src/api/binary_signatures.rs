use std::fs;
use std::io;

use super::Command;
use crate::print_json;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::utils::Style;
use crate::utils::StyledStr;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use crate::NonExistingPathSnafu;
use clap::Parser;
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use peridio_sdk::api::binary_signatures::CreateBinarySignatureParams;
use peridio_sdk::api::binary_signatures::CreateBinarySignatureResponse;
use peridio_sdk::api::binary_signatures::DeleteBinarySignatureParams;
use peridio_sdk::api::binary_signatures::ListBinarySignaturesParams;
use peridio_sdk::api::binary_signatures::ListBinarySignaturesResponse;
use peridio_sdk::api::Api;

use sha2::Digest;
use sha2::Sha256;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum BinarySignaturesCommand {
    Create(Box<Command<CreateCommand>>),
    Delete(Command<DeleteCommand>),
    List(Command<ListCommand>),
}

impl BinarySignaturesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The PRN of the binary to create a binary signature for.
    #[arg(
        long,
        short = 'b',
        value_parser = PRNValueParser::new(PRNType::Binary)
    )]
    pub binary_prn: String,
    /// The path of the file to automatically create a signature for. If you instead want to compute and provide the signature yourself, use the --signature option.
    #[arg(
        long,
        short = 'c',
        conflicts_with = "signature",
        required_unless_present = "signature"
    )]
    pub binary_content_path: Option<String>,
    /// The signature of the binary content.
    ///
    /// The hex encoded Ed25519 signature of the SHA256 hash of the binary content. To avoid computing this yourself, you can use the --binary-content-path option.
    #[arg(
        long,
        short = 'g',
        conflicts_with = "signing_key_private",
        conflicts_with = "binary_content_path",
        required_unless_present_any = ["signing_key_private", "binary_content_path"],
    )]
    pub signature: Option<String>,
    /// The name of a signing key pair as defined in your Peridio CLI config.
    ///
    /// If you instead want to provide the private key and PRN of the signing key yourself, use the --signing-key-private and --signing-key-prn options.
    #[arg(
        long,
        short = 's',
        conflicts_with = "signing_key_private",
        conflicts_with = "signing_key_prn",
        required_unless_present_any = ["signing_key_private", "signing_key_prn"],
    )]
    pub signing_key_pair: Option<String>,
    /// The path of the file containing the private key to sign the binary with.
    ///
    /// If you instead want to provide the name of a signing key pair as defined in your Peridio CLI config, use the --signing-key-pair option.
    #[arg(
        long,
        conflicts_with = "signature",
        conflicts_with = "signing_key_pair",
        required_unless_present_any = ["signature", "signing_key_pair"],
        requires = "binary_content_path",
    )]
    pub signing_key_private: Option<String>,
    /// The PRN of the signing key to tell Peridio to verify the signature with.
    ///
    /// If you instead want to provide the name of a signing key pair as defined in your Peridio CLI config, use the --signing-key-pair option.
    #[arg(
        long,
        conflicts_with = "signing_key_pair",
        required_unless_present = "signing_key_pair",
        value_parser = PRNValueParser::new(PRNType::SigningKey)
    )]
    pub signing_key_prn: Option<String>,

    #[clap(skip)]
    pub api: Option<Api>,

    #[clap(skip)]
    pub binary_content_hash: Option<String>,
}

impl CreateCommand {
    pub async fn run(
        self,
        global_options: GlobalOptions,
    ) -> Result<Option<CreateBinarySignatureResponse>, Error> {
        // user provides a signing_key_pair
        let (signing_key_prn, signature) = if let Some(signing_key_pair) = self.signing_key_pair {
            if let Some(signing_key_pairs) = global_options.signing_key_pairs.clone() {
                if let Some(key_pair) = signing_key_pairs.get(&signing_key_pair) {
                    // first we check for a binary path is provided
                    let signature = if let Some(binary_content_path) = self.binary_content_path {
                        Self::sign_binary(
                            key_pair.signing_key_private_path.clone(),
                            binary_content_path,
                            self.binary_content_hash.clone(),
                        )?
                    } else {
                        // otherwise the user must provide a signature
                        self.signature.unwrap()
                    };

                    (key_pair.signing_key_prn.clone(), signature)
                } else {
                    let mut error = StyledStr::new();
                    error.push_str(Some(Style::Error), "error: ".to_string());
                    error.push_str(None, "Config file field ".to_string());
                    error.push_str(None, "'".to_string());
                    error.push_str(
                        Some(Style::Warning),
                        format!("signing_key_pairs.{signing_key_pair}").to_string(),
                    );
                    error.push_str(None, "'".to_string());
                    error.push_str(
                        None,
                        " is unset or null, but is required by the --signing-key-pair option."
                            .to_string(),
                    );
                    error.print_data_err()
                }
            } else {
                let mut error = StyledStr::new();
                error.push_str(Some(Style::Error), "error: ".to_string());
                error.push_str(None, "Config file field ".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(Some(Style::Warning), "signing_key_pairs".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(
                    None,
                    " is unset or null, but is required by the --signing-key-pair option."
                        .to_string(),
                );
                error.print_data_err();
            }
        } else if let Some(signing_key_private_path) = self.signing_key_private {
            let binary_content_path = self.binary_content_path.unwrap();
            let signature = Self::sign_binary(
                signing_key_private_path,
                binary_content_path,
                self.binary_content_hash.clone(),
            )?;
            (self.signing_key_prn.unwrap(), signature)
        } else {
            (self.signing_key_prn.unwrap(), self.signature.unwrap())
        };

        let params = CreateBinarySignatureParams {
            binary_prn: self.binary_prn,
            signing_key_prn: Some(signing_key_prn),
            signature,
            signing_key_keyid: None,
        };

        let api = if let Some(api) = self.api {
            api
        } else {
            Api::from(global_options)
        };

        api.binary_signatures()
            .create(params)
            .await
            .context(ApiSnafu)
    }

    fn sign_binary(
        signing_key_private_path: String,
        binary_content_path: String,
        binary_content_hash: Option<String>,
    ) -> Result<String, Error> {
        // Expand environment variables in the path
        let expanded_path =
            shellexpand::full(&signing_key_private_path).map_err(|e| Error::Generic {
                error: format!("Failed to expand path '{signing_key_private_path}': {e}"),
            })?;

        let signing_key_private =
            fs::read_to_string(expanded_path.as_ref()).context(NonExistingPathSnafu {
                path: expanded_path.as_ref(),
            })?;

        let signing_key: SigningKey =
            SigningKey::from_pkcs8_pem(&signing_key_private).map_err(|e| Error::Generic {
                error: format!("Failed to parse private key from '{expanded_path}': {e}"),
            })?;

        let hash = if let Some(hash) = binary_content_hash {
            hash
        } else {
            let expanded_binary_path =
                shellexpand::full(&binary_content_path).map_err(|e| Error::Generic {
                    error: format!("Failed to expand binary path '{binary_content_path}': {e}"),
                })?;

            let mut binary_content =
                fs::File::open(expanded_binary_path.as_ref()).map_err(|e| Error::Generic {
                    error: format!("Failed to open binary file '{expanded_binary_path}': {e}"),
                })?;
            let mut hasher = Sha256::new();
            io::copy(&mut binary_content, &mut hasher).map_err(|e| Error::Generic {
                error: format!("Failed to read binary file '{expanded_binary_path}': {e}"),
            })?;
            let hash = hasher.finalize();
            format!("{hash:x}")
        };

        let signed_hash = signing_key.sign(hash.as_bytes());

        Ok(format!("{signed_hash:X}"))
    }
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self.inner.run(global_options).await? {
            Some(binary_signature) => print_json!(&binary_signature),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The PRN of the resource to delete.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::BinarySignature)
    )]
    binary_signature_prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteBinarySignatureParams {
            binary_signature_prn: self.inner.binary_signature_prn,
        };

        let api = Api::from(global_options);

        match api
            .binary_signatures()
            .delete(params)
            .await
            .context(ApiSnafu)?
        {
            Some(binary_signature) => print_json!(&binary_signature),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    /// Search string to filter binary signatures by binary_prn, signature, or keyid
    #[arg(long, short = 's')]
    pub search: Option<String>,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListBinarySignaturesParams {
            list: crate::utils::list::ListArgs {
                limit: None,
                order: None,
                search: self.inner.search,
                page: None,
            }
            .into(),
        };

        let api = Api::from(global_options);

        match api
            .binary_signatures()
            .list(params)
            .await
            .context(ApiSnafu)?
        {
            Some(response) => print_json!(&response),
            None => panic!(),
        }

        Ok(())
    }
}

/// Helper function to check if a binary signature already exists
pub async fn signature_exists(api: &Api, binary_prn: &str, keyid: &str) -> Result<bool, Error> {
    let params = ListBinarySignaturesParams {
        list: crate::utils::list::ListArgs {
            limit: None,
            order: None,
            search: Some(format!(
                "binary_prn:'{}' and signature.signing_key.keyid:'{}'",
                binary_prn, keyid
            )),
            page: None,
        }
        .into(),
    };

    match api
        .binary_signatures()
        .list(params)
        .await
        .context(ApiSnafu)?
    {
        Some(ListBinarySignaturesResponse {
            binary_signatures, ..
        }) => {
            // If we get any results, it means a signature exists for this binary_prn and keyid
            // The search already filtered for both values

            Ok(!binary_signatures.is_empty())
        }
        None => Ok(false),
    }
}
