use super::Command;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::bundle_overrides::{
    AddDeviceParams, CreateBundleOverrideParams, DeleteBundleOverrideParams, DeviceListParams,
    GetBundleOverrideParams, ListBundleOverridesParams, ListDevicesParams, RemoveDeviceParams,
    UpdateBundleOverrideParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum BundleOverridesCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
    Delete(Command<DeleteCommand>),
    #[command(subcommand)]
    Devices(DevicesCommand),
}

impl BundleOverridesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Devices(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub enum DevicesCommand {
    List(Command<ListDevicesCommand>),
    Add(Command<AddDeviceCommand>),
    Remove(Command<RemoveDeviceCommand>),
}

impl DevicesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Add(cmd) => cmd.run(global_options).await,
            Self::Remove(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The name of the bundle override.
    #[arg(long)]
    name: String,

    /// The PRN of the bundle to override.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    bundle_prn: String,

    /// The start date/time for the bundle override (ISO 8601 format).
    #[arg(long)]
    starts_at: String,

    /// The description of the bundle override.
    #[arg(long)]
    description: Option<String>,

    /// The end date/time for the bundle override (ISO 8601 format).
    #[arg(long)]
    ends_at: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateBundleOverrideParams {
            name: self.inner.name,
            bundle_prn: self.inner.bundle_prn,
            starts_at: self.inner.starts_at,
            description: self.inner.description,
            ends_at: self.inner.ends_at,
        };

        let api = Api::from(global_options);

        match api
            .bundle_overrides()
            .create(params)
            .await
            .context(ApiSnafu)?
        {
            Some(bundle_override) => print_json!(&bundle_override),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The PRN of the bundle override to delete.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::BundleOverride)
    )]
    prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteBundleOverrideParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        api.bundle_overrides()
            .delete(params)
            .await
            .context(ApiSnafu)?;

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
        let params = ListBundleOverridesParams {
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

        match api
            .bundle_overrides()
            .list(params)
            .await
            .context(ApiSnafu)?
        {
            Some(bundle_overrides) => print_json!(&bundle_overrides),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    /// The PRN of the bundle override to get.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::BundleOverride)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetBundleOverrideParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        match api.bundle_overrides().get(params).await.context(ApiSnafu)? {
            Some(bundle_override) => print_json!(&bundle_override),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// The PRN of the bundle override to update.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::BundleOverride)
    )]
    prn: String,

    /// The name of the bundle override.
    #[arg(long)]
    name: Option<String>,

    /// The description of the bundle override.
    #[arg(long)]
    description: Option<String>,

    /// The end date/time for the bundle override (ISO 8601 format).
    #[arg(long)]
    ends_at: Option<String>,

    /// The start date/time for the bundle override (ISO 8601 format).
    #[arg(long)]
    starts_at: Option<String>,

    /// The PRN of the bundle to override.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    bundle_prn: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateBundleOverrideParams {
            prn: self.inner.prn,
            name: self.inner.name,
            description: self.inner.description,
            ends_at: self.inner.ends_at,
            starts_at: self.inner.starts_at,
            bundle_prn: self.inner.bundle_prn,
        };

        let api = Api::from(global_options);

        match api
            .bundle_overrides()
            .update(params)
            .await
            .context(ApiSnafu)?
        {
            Some(response) => print_json!(&response),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListDevicesCommand {
    /// The PRN of the bundle override to list devices for.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::BundleOverride)
    )]
    prn: String,

    /// Limit the length of the page
    #[arg(long)]
    limit: Option<u8>,

    /// Specify whether the query is ordered ascending or descending
    #[arg(long)]
    order: Option<String>,

    /// A cursor for pagination across multiple pages of results. Don't include this parameter on the first call. Use the next_page value returned in a previous response (if not null) to request subsequent results
    #[arg(long)]
    page: Option<String>,
}

impl Command<ListDevicesCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListDevicesParams {
            prn: self.inner.prn,
            list: DeviceListParams {
                limit: self.inner.limit,
                order: self.inner.order,
                page: self.inner.page,
            },
        };

        let api = Api::from(global_options);

        match api
            .bundle_overrides()
            .list_devices(params)
            .await
            .context(ApiSnafu)?
        {
            Some(devices) => print_json!(&devices),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct AddDeviceCommand {
    /// The PRN of the bundle override to add a device to.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::BundleOverride)
    )]
    prn: String,

    /// The PRN of the device to add to the bundle override.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Device)
    )]
    device_prn: String,
}

impl Command<AddDeviceCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = AddDeviceParams {
            prn: self.inner.prn,
            device_prn: self.inner.device_prn,
        };

        let api = Api::from(global_options);

        match api
            .bundle_overrides()
            .add_device(params)
            .await
            .context(ApiSnafu)?
        {
            Some(response) => print_json!(&response),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct RemoveDeviceCommand {
    /// The PRN of the bundle override to remove a device from.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::BundleOverride)
    )]
    prn: String,

    /// The PRN of the device to remove from the bundle override.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Device)
    )]
    device_prn: String,
}

impl Command<RemoveDeviceCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = RemoveDeviceParams {
            prn: self.inner.prn,
            device_prn: self.inner.device_prn,
        };

        let api = Api::from(global_options);

        match api
            .bundle_overrides()
            .remove_device(params)
            .await
            .context(ApiSnafu)?
        {
            Some(response) => print_json!(&response),
            None => panic!(),
        }

        Ok(())
    }
}
