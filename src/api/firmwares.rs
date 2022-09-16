use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::firmwares::{
    CreateFirmwareParams, DeleteFirmwareParams, GetFirmwareParams, ListFirmwareParams,
};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;
use uuid::Uuid;

#[derive(StructOpt, Debug)]
pub enum FirmwaresCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
}

impl FirmwaresCommand {
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
    firmware: Option<String>,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(long)]
    ttl: Option<u32>,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = CreateFirmwareParams {
            firmware: self.inner.firmware,
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            ttl: self.inner.ttl,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.firmwares().create(params).await.context(ApiSnafu)? {
            Some(firmware) => print_json!(&firmware),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct DeleteCommand {
    #[structopt(long)]
    firmware_uuid: Uuid,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = DeleteFirmwareParams {
            firmware_uuid: self.inner.firmware_uuid.to_string(),
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.firmwares().delete(params).await.context(ApiSnafu)? {
            Some(_) => panic!(),
            None => (),
        };

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    #[structopt(long)]
    firmware_uuid: Uuid,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetFirmwareParams {
            firmware_uuid: self.inner.firmware_uuid.to_string(),
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.firmwares().get(params).await.context(ApiSnafu)? {
            Some(firmware) => print_json!(&firmware),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = ListFirmwareParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.firmwares().list(params).await.context(ApiSnafu)? {
            Some(firmwares) => print_json!(&firmwares),
            None => panic!(),
        }

        Ok(())
    }
}
