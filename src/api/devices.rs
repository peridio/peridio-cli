use std::fs;

use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::devices::{
    AuthenticateDeviceParams, CreateDeviceParams, DeleteDeviceParams, GetDeviceParams,
    ListDeviceParams, UpdateDeviceParams,
};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum DevicesCommand {
    Authenticate(Command<AuthenticateCommand>),
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
}

impl DevicesCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Authenticate(cmd) => cmd.run().await,
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Get(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
            Self::Update(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    #[structopt(long)]
    description: Option<String>,

    #[structopt(long)]
    healthy: Option<bool>,

    #[structopt(long)]
    identifier: String,

    #[structopt(long)]
    last_communication: Option<String>,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(long, required = true)]
    tags: Option<Vec<String>>,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = CreateDeviceParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            description: self.inner.description,
            healthy: self.inner.healthy,
            identifier: self.inner.identifier,
            last_communication: self.inner.last_communication,
            tags: self.inner.tags,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.devices().create(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct DeleteCommand {
    #[structopt(long)]
    device_identifier: String,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = DeleteDeviceParams {
            device_identifier: self.inner.device_identifier,
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        if (api.devices().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    #[structopt(long)]
    device_identifier: String,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetDeviceParams {
            device_identifier: self.inner.device_identifier,
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.devices().get(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
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
        let params = ListDeviceParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.devices().list(params).await.context(ApiSnafu)? {
            Some(devices) => print_json!(&devices),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct UpdateCommand {
    #[structopt(long)]
    description: Option<String>,

    #[structopt(long)]
    device_identifier: String,

    #[structopt(long)]
    healthy: Option<bool>,

    #[structopt(long)]
    last_communication: Option<String>,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(long)]
    tags: Option<Vec<String>>,
}

impl Command<UpdateCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = UpdateDeviceParams {
            device_identifier: self.inner.device_identifier,
            organization_name: self.inner.organization_name,
            description: self.inner.description,
            healthy: self.inner.healthy,
            last_communication: self.inner.last_communication,
            tags: self.inner.tags,
            product_name: self.inner.product_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.devices().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct AuthenticateCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(
        long,
        conflicts_with("certificate_path"),
        required_unless_one(&["certificate_path"])
    )]
    certificate: Option<String>,

    #[structopt(
        long,
        conflicts_with("certificate"),
        required_unless_one(&["certificate"])
    )]
    certificate_path: Option<String>,
}

impl Command<AuthenticateCommand> {
    async fn run(self) -> Result<(), Error> {
        let certificate = if let Some(cert_path) = self.inner.certificate_path {
            fs::read_to_string(cert_path).unwrap()
        } else {
            self.inner.certificate.unwrap()
        };
        let encoded_certificate = base64::encode(&certificate);

        let params = AuthenticateDeviceParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            certificate: encoded_certificate,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.devices().authenticate(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
