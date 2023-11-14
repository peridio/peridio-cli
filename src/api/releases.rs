use super::Command;
use crate::api::list::ListArgs;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::releases::{
    CreateReleaseParams, GetReleaseParams, ListReleasesParams, UpdateReleaseParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum ReleasesCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
}

impl ReleasesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]

pub struct CreateCommand {
    /// The PRN of the bundle you wish to release.
    #[arg(long)]
    bundle_prn: String,
    /// The PRN of the cohort you wish to create a release within.
    #[arg(long)]
    cohort_prn: String,
    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,
    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    name: String,

    /// The PRN of the release you wish to create this release before.
    ///
    /// If omitted, the release will be created as latest within the cohort. If there is already at least one release in the cohort, then the latest release in that cohort would have its next_release_prn updated to this created release.
    ///
    /// If supplied, the release will be created prior to the release identified by next_release_prn. If you wish to insert this release between two other releases, you may additionally supply previous_release_prn. If you supply neither field, it will create the release as the latest automatically.
    #[arg(long)]
    next_release_prn: Option<String>,
    /// The PRN of the organization you wish to create the resource within.
    #[arg(long)]
    organization_prn: String,
    /// The phase value controls the distribution of the update to your fleet.
    ///
    /// Decimals in [0.0, 1.0] are treated as percents, e.g., to allow 20% of the cohort to update, you would specifiy 0.2.
    ///
    /// Integers >= 2 are treated as absolute device counts, e.g., to allow 40 of the cohort's devices to update, you would specifiy 40.
    ///
    /// NOTE: 1 is a special value in that it represents 100% and once a release is updated to this value, the phase value can never be changed again.
    ///
    /// A release with a phase_value not equal to 1 is considered "phased".
    ///
    /// NOTE: There can only ever be a single release that is phased at a time within a cohort. Because of this, if there is already a phased release, it must be "completed" by setting the phase to 1.
    #[arg(long)]
    phase_value: f64,
    /// The PRN of the release you wish to create this release after.
    ///
    /// If omitted, next_release_prn will dictate where to create this release within the cohort's release graph.
    ///
    /// In order to insert a release between two other releases, next_release_prn is required to be supplied as well. If you supply neither field, it will create the release as the latest automatically.
    #[arg(long)]
    previous_release_prn: Option<String>,
    /// Whether the release is required.
    ///
    /// If true, this release must be passed through if encountered by a device.
    ///
    /// If false, this release will be skipped over when possible (if there are releases configured after it).
    #[arg(long)]
    required: bool,
    /// The date at which the release becomes available to devices.
    ///
    /// Before this date-time, the release will not be resolvable when checking for updates. You may use this to schedule a future release.
    #[arg(long)]
    schedule_date: String,
    /// The release version.
    ///
    /// If provided, it has to be a valid version. Used in dynamic release resolution.
    #[arg(long)]
    version: Option<String>,
    /// The release version requirement.
    ///
    /// If provided, it has to be a valid requirement. Used in dynamic release resolution.
    #[arg(long)]
    version_requirement: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateReleaseParams {
            bundle_prn: self.inner.bundle_prn,
            cohort_prn: self.inner.cohort_prn,
            description: self.inner.description,
            name: self.inner.name,
            organization_prn: self.inner.organization_prn,
            phase_value: self.inner.phase_value,
            required: self.inner.required,
            schedule_date: self.inner.schedule_date,
            next_release_prn: self.inner.next_release_prn,
            previous_release_prn: self.inner.previous_release_prn,
            version: self.inner.version,
            version_requirement: self.inner.version_requirement,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.releases().create(params).await.context(ApiSnafu)? {
            Some(release) => print_json!(&release),
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
        let params = ListReleasesParams {
            limit: self.inner.list_args.limit,
            order: self.inner.list_args.order,
            search: self.inner.list_args.search,
            page: self.inner.list_args.page,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.releases().list(params).await.context(ApiSnafu)? {
            Some(release) => print_json!(&release),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    /// The PRN of the resource to get.
    #[arg(long)]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetReleaseParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.releases().get(params).await.context(ApiSnafu)? {
            Some(release) => print_json!(&release),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// The PRN of the resource to update.
    #[arg(long)]
    pub prn: String,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    pub description: Option<String>,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    pub name: Option<String>,

    /// The PRN of the release you wish to create this release before.
    ///
    /// If omitted, the release will be created as latest within the cohort. If there is already at least one release in the cohort, then the latest release in that cohort would have its next_release_prn updated to this created release.
    ///
    /// If supplied, the release will be created prior to the release identified by next_release_prn. If you wish to insert this release between two other releases, you may additionally supply previous_release_prn. If you supply neither field, it will create the release as the latest automatically.
    #[arg(long)]
    pub next_release_prn: Option<String>,

    /// The phase value controls the distribution of the update to your fleet.
    ///
    /// Decimals in [0.0, 1.0] are treated as percents, e.g., to allow 20% of the cohort to update, you would specifiy 0.2.
    ///
    /// Integers >= 2 are treated as absolute device counts, e.g., to allow 40 of the cohort's devices to update, you would specifiy 40.
    ///
    /// NOTE: 1 is a special value in that it represents 100% and once a release is updated to this value, the phase value can never be changed again.
    ///
    /// A release with a phase_value not equal to 1 is considered "phased".
    ///
    /// NOTE: There can only ever be a single release that is phased at a time within a cohort. Because of this, if there is already a phased release, it must be "completed" by setting the phase to 1.
    #[arg(long)]
    pub phase_value: Option<f64>,

    /// Whether the release is required.
    ///
    /// If true, this release must be passed through if encountered by a device.
    ///
    /// If false, this release will be skipped over when possible (if there are releases configured after it).
    #[arg(long)]
    pub required: Option<bool>,

    /// The date at which the release becomes available to devices.
    ///
    /// Before this date-time, the release will not be resolvable when checking for updates. You may use this to schedule a future release.
    #[arg(long)]
    pub schedule_date: Option<String>,

    /// The release version.
    ///
    /// If provided, it has to be a valid version. Used in dynamic release resolution.
    #[arg(long)]
    version: Option<String>,

    /// The release version requirement.
    ///
    /// If provided, it has to be a valid requirement. Used in dynamic release resolution.
    #[arg(long)]
    version_requirement: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateReleaseParams {
            prn: self.inner.prn,
            description: self.inner.description,
            name: self.inner.name,
            next_release_prn: self.inner.next_release_prn,
            phase_value: self.inner.phase_value,
            required: self.inner.required,
            schedule_date: self.inner.schedule_date,
            version: self.inner.version,
            version_requirement: self.inner.version_requirement,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.releases().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
