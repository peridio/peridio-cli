use super::Command;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::devices::GetUpdateDeviceParams;
use peridio_sdk::api::devices::{
    CreateDeviceParams, DeleteDeviceParams, GetDeviceParams, ListDeviceParams, UpdateDeviceParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum DevicesCommand {
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

    /// Whether or not the device is quarantined or not.
    #[arg(long)]
    quarantined: Option<bool>,

    /// The device's identifier.
    #[arg(long)]
    identifier: String,

    /// The prn of the product you wish to create the resource within.
    #[arg(long)]
    product_prn: String,

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
            description: self.inner.description,
            quarantined: self.inner.quarantined,
            identifier: self.inner.identifier,
            tags: self.inner.tags,
            target: self.inner.target,
            cohort_prn: self.inner.cohort_prn,
            product_prn: self.inner.product_prn,
        };

        let api = Api::from(global_options);

        match api.devices().create(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The prn of the device you wish to delete.
    #[arg(long)]
    prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteDeviceParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        if (api.devices().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    /// The prn of the device you wish to get.
    #[arg(long)]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetDeviceParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        match api.devices().get(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
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
        let params = ListDeviceParams {
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

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
    prn: String,

    /// Whether or not the device is quarantined or not.
    #[arg(long)]
    quarantined: Option<bool>,

    /// The prn of the cohort you wish to update the resource within.
    #[arg(long)]
    cohort_prn: Option<String>,

    /// The prn of the product you wish to update the resource within.
    #[arg(long)]
    product_prn: Option<String>,

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
            prn: self.inner.prn,
            description: self.inner.description,
            quarantined: self.inner.quarantined,
            tags: self.inner.tags,
            cohort_prn: self.inner.cohort_prn,
            product_prn: self.inner.product_prn,
            target: self.inner.target,
        };

        let api = Api::from(global_options);

        match api.devices().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetUpdateCommand {
    /// The PRN of the device you wish to check for an update for.
    #[arg( long, value_parser = PRNValueParser::new(PRNType::Device) )]
    prn: String,

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
            prn: self.inner.prn,
            release_prn: self.inner.release_prn,
            bundle_prn: self.inner.bundle_prn,
            release_version: self.inner.release_version,
            write: self.inner.write,
        };

        let api = Api::from(global_options);

        match api.devices().get_update(params).await.context(ApiSnafu)? {
            Some(device_update) => print_json!(&device_update),
            None => panic!(),
        }

        Ok(())
    }
}
