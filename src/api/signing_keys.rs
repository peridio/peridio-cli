use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::signing_keys::{CreateParams, DeleteParams, GetParams, ListParams};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
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

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    #[structopt(long)]
    key: String,

    #[structopt(long)]
    name: String,

    #[structopt(long)]
    organization_name: String,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = CreateParams {
            key: self.inner.key,
            name: self.inner.name,
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.signing_keys().create(params).await.context(ApiSnafu)? {
            Some(key) => print_json!(&key),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct DeleteCommand {
    #[structopt(long)]
    name: String,

    #[structopt(long)]
    organization_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = DeleteParams {
            name: self.inner.name,
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        if (api.signing_keys().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    #[structopt(long)]
    name: String,

    #[structopt(long)]
    organization_name: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetParams {
            name: self.inner.name,
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.signing_keys().get(params).await.context(ApiSnafu)? {
            Some(key) => print_json!(&key),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {
    #[structopt(long)]
    organization_name: String,
}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = ListParams {
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.signing_keys().list(params).await.context(ApiSnafu)? {
            Some(signing_keys) => print_json!(&signing_keys),
            None => panic!(),
        }

        Ok(())
    }
}
