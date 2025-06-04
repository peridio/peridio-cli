use std::fs;
use std::io::BufWriter;
use std::io::Write;

use super::Command;
use crate::config::config_v2::ConfigV2;
use crate::config::config_v2::SigningKeyPairV2;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::utils::maybe_config_directory;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::AlreadyExistingFileSnafu;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use crate::NonExistingPathSnafu;
use base64::engine::general_purpose;
use base64::Engine;
use clap::Parser;
use ed25519_dalek::pkcs8::DecodePublicKey;
use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::pkcs8::EncodePublicKey;
use ed25519_dalek::SigningKey;
use ed25519_dalek::VerifyingKey;
use peridio_sdk::api::signing_keys::CreateSigningKeyParams;
use peridio_sdk::api::signing_keys::DeleteSigningKeyParams;
use peridio_sdk::api::signing_keys::GetSigningKeyParams;
use peridio_sdk::api::signing_keys::ListSigningKeysParams;
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
use snafu::ensure;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum SigningKeysCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
}

impl SigningKeysCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
        }
    }
}

/// Creates a new signing key resource in the Peridio Admin API.
/// You can either generate a new signing key by specifying a supported algorithm (using --algorithm and --out together)
/// or provide an existing public key using one of the mutually exclusive options (--value, --key, or --path).
/// Additionally, if you use --config, the generated key is automatically added to your CLI configuration under the signing-key-pairs section.
#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The public key raw file contents. --value cannot be used with --algorithm, --key, --out, or --path.
    #[arg(
        long,
        conflicts_with = "key",
        conflicts_with = "path",
        conflicts_with = "algorithm",
        required_unless_present = "key",
        required_unless_present = "path",
        required_unless_present = "algorithm"
    )]
    value: Option<String>,

    /// The name to create the key in the Peridio Admin API with.
    #[arg(long)]
    name: String,

    /// The path to the public key raw file. --key cannot be used with --algorithm, --out, --path, or --value.
    #[arg(
        long,
        conflicts_with = "value",
        conflicts_with = "path",
        conflicts_with = "algorithm",
        conflicts_with = "out",
        required_unless_present = "value",
        required_unless_present = "path",
        required_unless_present = "algorithm",
        required_unless_present = "out"
    )]
    key: Option<String>,

    /// The path to the public key pem file.
    #[arg(
        long,
        conflicts_with = "key",
        conflicts_with = "value",
        conflicts_with = "algorithm",
        required_unless_present = "key",
        required_unless_present = "value",
        required_unless_present = "algorithm"
    )]
    path: Option<String>,

    /// Specifies the cryptographic algorithm to use for generating the signing key.
    /// Note that only Ed25519 is supported at this time. --algorithm must be used together with --out.
    /// --algorithm cannot be used with --value, --key, or --path.
    #[arg(
        long,
        requires = "out",
        conflicts_with = "key",
        conflicts_with = "value",
        conflicts_with = "path",
        required_unless_present = "key",
        required_unless_present = "value",
        required_unless_present = "path",
        value_parser(clap::builder::PossibleValuesParser::new(["Ed25519"]))
    )]
    algorithm: Option<String>,

    /// Specifies the output directory where the generated key will be saved.
    /// --out must be used together with --algorithm.
    /// --out cannot be used with --key, --path, or --value.
    #[arg(
        long,
        requires = "algorithm",
        conflicts_with = "key",
        conflicts_with = "value",
        conflicts_with = "path",
        required_unless_present = "key",
        required_unless_present = "value",
        required_unless_present = "path"
    )]
    out: Option<String>,

    /// Automatically configures the generated signing key into your CLI configuration's signing-key-pairs section.
    #[arg(
        long,
        requires = "algorithm",
        requires = "out",
        conflicts_with = "key",
        conflicts_with = "value",
        conflicts_with = "path"
    )]
    config: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let (value, priv_path) = if self.inner.algorithm.is_some() {
            let out = self.inner.out.unwrap();

            let out_path = crate::utils::path_from(&out).expect("Invalid provided out path");
            fs::create_dir_all(&out_path).unwrap();

            let mut csprng = rand_core::OsRng;
            let signing_key: SigningKey = SigningKey::generate(&mut csprng);
            let verifying_key = signing_key.verifying_key();

            let out_priv_pem = out_path.join("private.pem");
            ensure!(
                !out_priv_pem.exists(),
                AlreadyExistingFileSnafu {
                    path: &out_priv_pem
                }
            );

            signing_key
                .write_pkcs8_pem_file(
                    &out_priv_pem,
                    ed25519_dalek::pkcs8::spki::der::pem::LineEnding::LF,
                )
                .unwrap_or_else(|_| {
                    panic!("Can't save private key to {}", &out_priv_pem.display())
                });

            let out_pub_pem = out_path.join("public.pem");
            ensure!(
                !out_pub_pem.exists(),
                AlreadyExistingFileSnafu { path: &out_pub_pem }
            );
            verifying_key
                .write_public_key_pem_file(
                    &out_pub_pem,
                    ed25519_dalek::pkcs8::spki::der::pem::LineEnding::LF,
                )
                .unwrap_or_else(|_| panic!("Can't save public key to {}", &out_pub_pem.display()));
            let pub_key_bytes = verifying_key.as_bytes();

            let value = general_purpose::STANDARD.encode(pub_key_bytes);

            (value, Some(out_priv_pem.to_str().unwrap().to_string()))
        } else if let Some(path) = self.inner.path {
            let verifying_key_pub =
                fs::read_to_string(&path).context(NonExistingPathSnafu { path: &path })?;
            let verifying_key = VerifyingKey::from_public_key_pem(&verifying_key_pub)
                .expect("invalid public key PEM");

            let raw_bytes = verifying_key.as_bytes();

            (general_purpose::STANDARD.encode(raw_bytes), None)
        } else if let Some(key) = self.inner.key {
            let value = fs::read_to_string(&key)
                .context(NonExistingPathSnafu { path: &key })?
                .trim()
                .to_owned();
            (value, None)
        } else {
            (self.inner.value.unwrap(), None)
        };

        let params = CreateSigningKeyParams {
            value,
            name: self.inner.name,
        };

        let api = Api::from(global_options.clone());

        match api.signing_keys().create(params).await.context(ApiSnafu)? {
            Some(key) => {
                // handle config output
                if let Some(signing_key_key) = self.inner.config {
                    let config_dir_path = if let Some(config_dir) =
                        maybe_config_directory(&global_options)
                    {
                        config_dir
                    } else {
                        panic!("provided --config but we can't determine your peridio config path");
                    };
                    fs::create_dir_all(&config_dir_path).unwrap();

                    let config_path = config_dir_path.join("config.json");
                    let mut config = if config_path.exists() {
                        ConfigV2::try_from(&config_path).unwrap_or_default()
                    } else {
                        ConfigV2::default()
                    };

                    let mut signing_key_pairs =
                        config.signing_key_pairs.clone().unwrap_or_default();

                    signing_key_pairs.insert(
                        signing_key_key,
                        SigningKeyPairV2 {
                            signing_key_prn: key.signing_key.prn.clone().to_owned(),
                            signing_key_private_path: priv_path.unwrap(),
                        },
                    );

                    config.signing_key_pairs = signing_key_pairs.into();

                    // Write config file
                    let file = std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&config_path)
                        .unwrap();
                    let mut writer = BufWriter::new(file);
                    serde_json::to_writer_pretty(&mut writer, &config).unwrap();
                    writer.flush().unwrap();
                }
                print_json!(&key)
            }
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    /// The PRN of the resource to get.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::SigningKey)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetSigningKeyParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        match api.signing_keys().get(params).await.context(ApiSnafu)? {
            Some(key) => print_json!(&key),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[clap(flatten)]
    list_args: ListArgs,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListSigningKeysParams {
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

        match api.signing_keys().list(params).await.context(ApiSnafu)? {
            Some(signing_key) => print_json!(&signing_key),
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
        value_parser = PRNValueParser::new(PRNType::SigningKey)
    )]
    signing_key_prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteSigningKeyParams {
            signing_key_prn: self.inner.signing_key_prn,
        };

        let api = Api::from(global_options);

        if (api.signing_keys().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}
