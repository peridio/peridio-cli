use std::fs;
use std::io;

use super::Command;
use crate::print_json;
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
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use sha2::Digest;
use sha2::Sha256;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum BinarySignaturesCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
}

impl BinarySignaturesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    #[arg(
        long,
        short = 'b',
        help = "The PRN of the binary this signature will be associated with."
    )]
    pub binary_prn: String,
    #[arg(
        long,
        short = 'c',
        conflicts_with = "signature",
        required_unless_present = "signature",
        help = "The path of the file to create a signature for."
    )]
    pub binary_content_path: Option<String>,
    #[arg(
        long,
        short = 'g',
        conflicts_with = "signing_key_private",
        conflicts_with = "binary_content_path",
        required_unless_present_any = ["signing_key_private", "binary_content_path"],
        help = "The hex encoded signature of the SHA256 hash of the binary content."
    )]
    pub signature: Option<String>,
    #[arg(
        long,
        short = 's',
        conflicts_with = "signing_key_private",
        conflicts_with = "signing_key_prn",
        required_unless_present_any = ["signing_key_private", "signing_key_prn"],
        help = "The name of a signing key pair as defined in your Peridio CLI config."
    )]
    pub signing_key_pair: Option<String>,
    #[arg(
        long,
        conflicts_with = "signature",
        conflicts_with = "signing_key_pair",
        required_unless_present_any = ["signature", "signing_key_pair"],
        requires = "binary_content_path",
        help = "The PEM base64-encoded PKCS #8 private key."
    )]
    pub signing_key_private: Option<String>,
    #[arg(
        long,
        conflicts_with = "signing_key_pair",
        required_unless_present = "signing_key_pair",
        help = "The PRN of the signing key to tell Peridio to verify the signature with."
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
            if let Some(key_pair) = global_options
                .signing_key_pairs
                .unwrap()
                .get(&signing_key_pair)
            {
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
                error.push_str(None, "Signing Key ".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(Some(Style::Warning), signing_key_pair.to_string());
                error.push_str(None, "'".to_string());
                error.push_str(None, " not found.".to_string());
                error.print_data_err()
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
            signing_key_prn,
            signature,
        };

        let api = if let Some(api) = self.api {
            api
        } else {
            Api::new(ApiOptions {
                api_key: global_options.api_key.unwrap(),
                endpoint: global_options.base_url,
                ca_bundle_path: global_options.ca_path,
            })
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
        let signing_key_private =
            fs::read_to_string(&signing_key_private_path).context(NonExistingPathSnafu {
                path: &signing_key_private_path,
            })?;
        let signing_key: SigningKey = SigningKey::from_pkcs8_pem(&signing_key_private).unwrap();

        let hash = if let Some(hash) = binary_content_hash {
            hash
        } else {
            let mut binary_content = fs::File::open(binary_content_path).unwrap();
            let mut hasher = Sha256::new();
            let _ = io::copy(&mut binary_content, &mut hasher).unwrap();
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
    #[arg(long)]
    binary_signature_prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteBinarySignatureParams {
            binary_signature_prn: self.inner.binary_signature_prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

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
