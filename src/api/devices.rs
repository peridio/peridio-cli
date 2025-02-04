use std::fs;

use super::Command;
use crate::print_json;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use peridio_sdk::api::devices::GetUpdateDeviceParams;
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
    GetUpdate(Command<GetUpdateCommand>),
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
            Self::GetUpdate(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,

    /// Whether or not the device is healthy (quarantined or not).
    #[arg(long)]
    healthy: Option<bool>,

    /// The device's identifier.
    #[arg(long)]
    identifier: String,

    /// The device's last communication time.
    #[arg(long)]
    last_communication: Option<String>,

    /// The name of the product you wish to create the resource within.
    #[arg(long)]
    product_name: String,

    /// A list of tags to attach to the device.
    ///
    /// If using firmwares and deployments, tags can be used to target devices.
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    tags: Option<Vec<String>>,

    /// The target of the device.
    ///
    /// Commonly used to store the device's target triplet to indicate architecture/compiler compatibility.
    #[arg(long)]
    target: Option<String>,

    /// The PRN of the cohort you wish to add the device to.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Cohort)
    )]
    cohort_prn: Option<String>,
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
            target: self.inner.target,
            cohort_prn: self.inner.cohort_prn,
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
    /// The identifier of the device you wish to delete.
    #[arg(long)]
    device_identifier: String,

    /// The name of the product you wish to delete the resource within.
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
    /// The identifier of the device you wish to get.
    #[arg(long)]
    device_identifier: String,

    /// The name of the product you wish to get the resource within.
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
    /// The name of the product
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
    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,

    /// The identifier of the device you wish to update.
    #[arg(long)]
    device_identifier: String,

    /// Whether or not the device is healthy (quarantined or not).
    #[arg(long)]
    healthy: Option<bool>,

    /// The device's last communication time.
    #[arg(long)]
    last_communication: Option<String>,

    /// The name of the product you wish to update the resource within.
    #[arg(long)]
    product_name: String,

    /// A list of tags to attach to the device.
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    tags: Option<Vec<String>>,

    /// The target of the device.
    #[arg(long)]
    target: Option<String>,
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
            target: self.inner.target,
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
    /// The name of the product you wish to authenticate the device within.
    #[arg(long)]
    product_name: String,

    /// The certificate of the device you wish to authenticate.
    #[arg(
        long,
        conflicts_with("certificate_path"),
        required_unless_present("certificate_path")
    )]
    certificate: Option<String>,

    /// The path to the certificate of the device you wish to authenticate.
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
        let encoded_certificate = general_purpose::STANDARD.encode(&certificate);

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

#[derive(Parser, Debug)]
pub struct GetUpdateCommand {
    /// The PRN of the device you wish to check for an update for.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Device)
    )]
    device_prn: String,

    /// The PRN of the release to consider as the device's current release during bundle resolution.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Release),
        required_unless_present_any = ["bundle_prn", "release_version"]
    )]
    release_prn: Option<String>,

    /// The PRN of the bundle to consider as the device's current bundle during bundle resolution.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Bundle),
        required_unless_present_any = ["release_prn", "release_version"]
    )]
    bundle_prn: Option<String>,

    /// The version to consider as the device's current release version during bundle resolution.
    #[arg(long, required_unless_present_any = ["release_prn", "bundle_prn"])]
    release_version: Option<String>,

    /// Whether the server's record of what the device's current state is will be updated in reaction to the release PRN, bundle PRN, and release version parameters if they are also supplied.
    #[arg(long, default_value = "false")]
    write: bool,
}

impl Command<GetUpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetUpdateDeviceParams {
            device_prn: self.inner.device_prn,
            release_prn: self.inner.release_prn,
            bundle_prn: self.inner.bundle_prn,
            release_version: self.inner.release_version,
            write: self.inner.write,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.devices().get_update(params).await.context(ApiSnafu)? {
            Some(device_update) => print_json!(&device_update),
            None => panic!(),
        }

        Ok(())
    }
}
