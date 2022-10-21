use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use clap::Parser;
use peridio_sdk::api::firmwares::{
    CreateFirmwareParams, DeleteFirmwareParams, GetFirmwareParams, ListFirmwareParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;
use uuid::Uuid;

#[derive(Parser, Debug)]
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

#[derive(Parser, Debug)]
pub struct CreateCommand {
    #[arg(long)]
    firmware_path: String,

    #[arg(long)]
    product_name: String,

    #[arg(long)]
    ttl: Option<u32>,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = CreateFirmwareParams {
            firmware_path: self.inner.firmware_path,
            organization_name: self.organization_name,
            product_name: self.inner.product_name,
            ttl: self.inner.ttl,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api.firmwares().create(params).await.context(ApiSnafu)? {
            Some(firmware) => print_json!(&firmware),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    #[arg(long)]
    firmware_uuid: Uuid,

    #[arg(long)]
    product_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = DeleteFirmwareParams {
            firmware_uuid: self.inner.firmware_uuid.to_string(),
            organization_name: self.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        if (api.firmwares().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    firmware_uuid: Uuid,

    #[arg(long)]
    product_name: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetFirmwareParams {
            firmware_uuid: self.inner.firmware_uuid.to_string(),
            organization_name: self.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api.firmwares().get(params).await.context(ApiSnafu)? {
            Some(firmware) => print_json!(&firmware),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[arg(long)]
    product_name: String,
}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = ListFirmwareParams {
            organization_name: self.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api.firmwares().list(params).await.context(ApiSnafu)? {
            Some(firmwares) => print_json!(&firmwares),
            None => panic!(),
        }

        Ok(())
    }
}
