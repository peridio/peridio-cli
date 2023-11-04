use std::fs;

use super::Command;
use crate::api::list::ListArgs;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use crate::NonExistingPathSnafu;
use base64::engine::general_purpose;
use base64::Engine;
use clap::Parser;
use ed25519_dalek::pkcs8::DecodePublicKey;
use ed25519_dalek::VerifyingKey;
use peridio_sdk::api::signing_keys::CreateSigningKeyParams;
use peridio_sdk::api::signing_keys::DeleteSigningKeyParams;
use peridio_sdk::api::signing_keys::GetSigningKeyParams;
use peridio_sdk::api::signing_keys::ListSigningKeysParams;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
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

#[derive(Parser, Debug)]
pub struct CreateCommand {
    #[arg(
        long,
        conflicts_with = "key",
        conflicts_with = "path",
        required_unless_present = "key",
        required_unless_present = "path"
    )]
    value: Option<String>,
    #[arg(long)]
    name: String,
    #[arg(long)]
    organization_prn: String,
    #[arg(
        long,
        conflicts_with = "value",
        conflicts_with = "path",
        required_unless_present = "value",
        required_unless_present = "path",
        help = "The path to the public key raw file."
    )]
    key: Option<String>,
    #[arg(
        long,
        conflicts_with = "key",
        conflicts_with = "value",
        required_unless_present = "key",
        required_unless_present = "value",
        help = "The path to the public key pem file."
    )]
    path: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let value = if let Some(path) = self.inner.path {
            let verifying_key_pub =
                fs::read_to_string(&path).context(NonExistingPathSnafu { path: &path })?;
            let verifying_key = VerifyingKey::from_public_key_pem(&verifying_key_pub)
                .expect("invalid public key PEM");

            let raw_bytes = verifying_key.as_bytes();

            general_purpose::STANDARD.encode(raw_bytes)
        } else if let Some(key) = self.inner.key {
            fs::read_to_string(&key)
                .context(NonExistingPathSnafu { path: &key })?
                .trim()
                .to_owned()
        } else {
            self.inner.value.unwrap()
        };

        let params = CreateSigningKeyParams {
            value,
            name: self.inner.name,
            organization_prn: self.inner.organization_prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.signing_keys().create(params).await.context(ApiSnafu)? {
            Some(key) => print_json!(&key),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetSigningKeyParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

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
            limit: self.inner.list_args.limit,
            order: self.inner.list_args.order,
            search: self.inner.list_args.search,
            page: self.inner.list_args.page,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.signing_keys().list(params).await.context(ApiSnafu)? {
            Some(signing_key) => print_json!(&signing_key),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    #[arg(long)]
    signing_key_prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteSigningKeyParams {
            signing_key_prn: self.inner.signing_key_prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        if (api.signing_keys().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}
