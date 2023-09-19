use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
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
    #[arg(long)]
    value: String,
    #[arg(long)]
    name: String,
    #[arg(long)]
    organization_prn: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateSigningKeyParams {
            value: self.inner.value,
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
    #[arg(long)]
    pub limit: Option<u8>,
    #[arg(long)]
    pub order: Option<String>,
    #[arg(long)]
    pub search: String,
    #[arg(long)]
    pub page: Option<String>,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListSigningKeysParams {
            limit: self.inner.limit,
            order: self.inner.order,
            search: self.inner.search,
            page: self.inner.page,
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
