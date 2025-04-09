use super::Command;
use crate::print_json;
use crate::utils::sdk_extensions::ApiExt;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::firmwares::{
    CreateFirmwareParams, DeleteFirmwareParams, GetFirmwareParams, ListFirmwareParams,
};
use peridio_sdk::api::Api;
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
    /// The path to the firmware's binary content.
    #[arg(long)]
    firmware_path: String,

    /// The name of the product you wish to create the resource within.
    #[arg(long)]
    product_name: String,

    /// The time-to-live of the firmware in seconds.
    ///
    /// This is the amount of time the firmware can go without being associated to neither a deployment nor a device. After this time, the firmware will be deleted.
    #[arg(long)]
    ttl: Option<u32>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateFirmwareParams {
            firmware_path: self.inner.firmware_path,
            organization_name: global_options
                .organization_name
                .as_ref()
                .unwrap()
                .to_string(),
            product_name: self.inner.product_name,
            ttl: self.inner.ttl,
        };

        let api = Api::from_options(global_options);

        match api.firmwares().create(params).await.context(ApiSnafu)? {
            Some(firmware) => print_json!(&firmware),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The UUID of the firmware you wish to delete.
    #[arg(long)]
    firmware_uuid: Uuid,

    /// The name of the product you wish to delete the resource within.
    #[arg(long)]
    product_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteFirmwareParams {
            firmware_uuid: self.inner.firmware_uuid.to_string(),
            organization_name: global_options
                .organization_name
                .as_ref()
                .unwrap()
                .to_string(),
            product_name: self.inner.product_name,
        };

        let api = Api::from_options(global_options);

        if (api.firmwares().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    /// The UUID of the firmware you wish to get.
    #[arg(long)]
    firmware_uuid: Uuid,

    /// The name of the product you wish to get the resource within.
    #[arg(long)]
    product_name: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetFirmwareParams {
            firmware_uuid: self.inner.firmware_uuid.to_string(),
            organization_name: global_options
                .organization_name
                .as_ref()
                .unwrap()
                .to_string(),
            product_name: self.inner.product_name,
        };

        let api = Api::from_options(global_options);

        match api.firmwares().get(params).await.context(ApiSnafu)? {
            Some(firmware) => print_json!(&firmware),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    /// The name of the product you wish to list the resources within.
    #[arg(long)]
    product_name: String,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListFirmwareParams {
            organization_name: global_options
                .organization_name
                .as_ref()
                .unwrap()
                .to_string(),
            product_name: self.inner.product_name,
        };

        let api = Api::from_options(global_options);

        match api.firmwares().list(params).await.context(ApiSnafu)? {
            Some(firmwares) => print_json!(&firmwares),
            None => panic!(),
        }

        Ok(())
    }
}
