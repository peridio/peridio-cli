use std::fs;

use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::devices::{
    AuthenticateDeviceParams, CreateDeviceParams, DeleteDeviceParams, GetDeviceParams,
    ListDeviceParams, UpdateDeviceParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum DevicesCommand {
    Authenticate(Command<AuthenticateCommand>),
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
}

impl DevicesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Authenticate(cmd) => cmd.run(global_options).await,
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    #[arg(long)]
    description: Option<String>,

    #[arg(long)]
    healthy: Option<bool>,

    #[arg(long)]
    identifier: String,

    #[arg(long)]
    last_communication: Option<String>,

    #[arg(long)]
    product_name: String,

    #[arg(long)]
    tags: Option<Vec<String>>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateDeviceParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            description: self.inner.description,
            healthy: self.inner.healthy,
            identifier: self.inner.identifier,
            last_communication: self.inner.last_communication,
            tags: self.inner.tags,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.devices().create(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    #[arg(long)]
    device_identifier: String,

    #[arg(long)]
    product_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteDeviceParams {
            device_identifier: self.inner.device_identifier,
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        if (api.devices().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    device_identifier: String,

    #[arg(long)]
    product_name: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetDeviceParams {
            device_identifier: self.inner.device_identifier,
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.devices().get(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
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
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListDeviceParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.devices().list(params).await.context(ApiSnafu)? {
            Some(devices) => print_json!(&devices),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    #[arg(long)]
    description: Option<String>,

    #[arg(long)]
    device_identifier: String,

    #[arg(long)]
    healthy: Option<bool>,

    #[arg(long)]
    last_communication: Option<String>,

    #[arg(long)]
    product_name: String,

    #[arg(long)]
    tags: Option<Vec<String>>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateDeviceParams {
            device_identifier: self.inner.device_identifier,
            organization_name: global_options.organization_name.unwrap(),
            description: self.inner.description,
            healthy: self.inner.healthy,
            last_communication: self.inner.last_communication,
            tags: self.inner.tags,
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.devices().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct AuthenticateCommand {
    #[arg(long)]
    product_name: String,

    #[arg(
        long,
        conflicts_with("certificate_path"),
        required_unless_present("certificate_path")
    )]
    certificate: Option<String>,

    #[arg(
        long,
        conflicts_with("certificate"),
        required_unless_present("certificate")
    )]
    certificate_path: Option<String>,
}

impl Command<AuthenticateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let certificate = if let Some(cert_path) = self.inner.certificate_path {
            fs::read_to_string(cert_path).unwrap()
        } else {
            self.inner.certificate.unwrap()
        };
        let encoded_certificate = base64::encode(&certificate);

        let params = AuthenticateDeviceParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            certificate: encoded_certificate,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.devices().authenticate(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
