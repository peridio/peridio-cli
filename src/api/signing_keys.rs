use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use clap::Parser;
use peridio_sdk::api::signing_keys::{CreateParams, DeleteParams, GetParams, ListParams};
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
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Get(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    #[arg(long)]
    key: String,

    #[arg(long)]
    name: String,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = CreateParams {
            key: self.inner.key,
            name: self.inner.name,
            organization_name: self.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api.signing_keys().create(params).await.context(ApiSnafu)? {
            Some(key) => print_json!(&key),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    #[arg(long)]
    name: String,
}

impl Command<DeleteCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = DeleteParams {
            name: self.inner.name,
            organization_name: self.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        if (api.signing_keys().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    name: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetParams {
            name: self.inner.name,
            organization_name: self.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api.signing_keys().get(params).await.context(ApiSnafu)? {
            Some(key) => print_json!(&key),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = ListParams {
            organization_name: self.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api.signing_keys().list(params).await.context(ApiSnafu)? {
            Some(signing_keys) => print_json!(&signing_keys),
            None => panic!(),
        }

        Ok(())
    }
}
