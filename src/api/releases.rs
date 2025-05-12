use super::Command;
use crate::api::list::ListArgs;
use crate::print_json;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::releases::{
    CreateReleaseParams, DeleteReleaseParams, GetReleaseParams, ListReleasesParams,
    UpdateReleaseParams,
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
    Delete(Command<DeleteCommand>),
}

impl ReleasesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]

pub struct CreateCommand {
    /// The PRN of the bundle you wish to release.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    bundle_prn: String,
    /// The PRN of the cohort you wish to create a release within.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Cohort)
    )]
    cohort_prn: String,
    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,
    /// If a release is marked as disabled it cannot be resolved during release resolution.
    #[arg(long)]
    pub disabled: Option<bool>,
    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    name: String,

    /// The PRN of the release you wish to create this release before.
    ///
    /// If omitted, the release will be created as latest within the cohort. If there is already at least one release in the cohort, then the latest release in that cohort would have its next_release_prn updated to this created release.
    ///
    /// If supplied, the release will be created prior to the release identified by next_release_prn. If you wish to insert this release between two other releases, you may additionally supply previous_release_prn. If you supply neither field, it will create the release as the latest automatically.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Release)
    )]
    next_release_prn: Option<String>,
    /// The PRN of the organization you wish to create the resource within.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Organization)
    )]
    organization_prn: String,

    /// Limits by tags the devices that are allowed to update to this release.
    /// When phase_mode is tags, this field only allows devices to update to this release if they have at least one of these tags.
    ///
    /// Values can be provided by passing each value in a flag
    /// or by delimiting all values with ","
    #[arg(
        long,
        conflicts_with = "phase_value",
        required_unless_present_any = ["phase_value"],
        num_args = 0..,
        value_delimiter = ',',
    )]
    phase_tags: Option<Vec<String>>,

    /// The phase value controls the distribution of the update to your fleet.
    ///
    /// Decimals in [0.0, 1.0] are treated as percents, e.g., to allow 20% of the cohort to update, you would specify 0.2.
    ///
    /// Integers >= 2 are treated as absolute device counts, e.g., to allow 40 of the cohort's devices to update, you would specifiy 40.
    ///
    /// NOTE: 1 is a special value in that it represents 100% and once a release is updated to this value, the phase value can never be changed again.
    ///
    /// A release with a phase_value not equal to 1 is considered "phased".
    #[arg(
        long,
        conflicts_with = "phase_tags",
        required_unless_present_any = ["phase_tags"],
    )]
    phase_value: Option<f64>,
    /// The PRN of the release you wish to create this release after.
    ///
    /// If omitted, next_release_prn will dictate where to create this release within the cohort's release graph.
    ///
    /// In order to insert a release between two other releases, next_release_prn is required to be supplied as well. If you supply neither field, it will create the release as the latest automatically.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Release)
    )]
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
        let (phase_mode, phase_value, phase_tags) = if let Some(phase_tags) = self.inner.phase_tags
        {
            ("tags".into(), None, Some(phase_tags))
        } else {
            ("numeric".into(), self.inner.phase_value, None)
        };

        let params = CreateReleaseParams {
            bundle_prn: self.inner.bundle_prn,
            cohort_prn: self.inner.cohort_prn,
            description: self.inner.description,
            disabled: self.inner.disabled,
            name: self.inner.name,
            organization_prn: self.inner.organization_prn,
            phase_mode: Some(phase_mode),
            phase_tags,
            phase_value,
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
pub struct DeleteCommand {
    /// The PRN of the resource to delete.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Release)
    )]
    prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteReleaseParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        api.releases().delete(params).await.context(ApiSnafu)?;

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
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Release)
    )]
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
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Release)
    )]
    pub prn: String,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    pub description: Option<String>,

    /// If a release is marked as disabled it cannot be resolved during release resolution.
    #[arg(long)]
    pub disabled: Option<bool>,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    pub name: Option<String>,

    /// The PRN of the release you wish to create this release before.
    ///
    /// If omitted, the release will be created as latest within the cohort. If there is already at least one release in the cohort, then the latest release in that cohort would have its next_release_prn updated to this created release.
    ///
    /// If supplied, the release will be created prior to the release identified by next_release_prn. If you wish to insert this release between two other releases, you may additionally supply previous_release_prn. If you supply neither field, it will create the release as the latest automatically.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Release)
    )]
    pub next_release_prn: Option<String>,

    /// Describes if this release is using tag or numeric based phasing. tags or phase value for resolution
    ///
    /// - tags - Phases rollout of the release according to the phase_tags field.
    ///
    /// - numeric - Phases rollout of the release according to the phase_value field.
    #[arg(long, value_parser(clap::builder::PossibleValuesParser::new(["tags", "numeric"])))]
    pub phase_mode: Option<String>,

    /// Limits by tags the devices that are allowed to update to this release.
    /// When phase_mode is tags, this field only allows devices to update to this release if they have at least one of these tags.
    ///
    /// Values can be provided by passing each value in a flag
    /// or by delimiting all values with ","
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    pub phase_tags: Option<Vec<String>>,

    /// The phase value controls the distribution of the update to your fleet.
    ///
    /// Decimals in [0.0, 1.0] are treated as percents, e.g., to allow 20% of the cohort to update, you would specify 0.2.
    ///
    /// Integers >= 2 are treated as absolute device counts, e.g., to allow 40 of the cohort's devices to update, you would specifiy 40.
    ///
    /// NOTE: 1 is a special value in that it represents 100% and once a release is updated to this value, the phase value can never be changed again.
    ///
    /// A release with a phase_value not equal to 1 is considered "phased".
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
            disabled: self.inner.disabled,
            name: self.inner.name,
            next_release_prn: self.inner.next_release_prn,
            phase_mode: self.inner.phase_mode,
            phase_tags: self.inner.phase_tags,
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
