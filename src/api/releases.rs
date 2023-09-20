use super::Command;
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
    #[arg(long)]
    bundle_prn: String,
    #[arg(long)]
    cohort_prn: String,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    name: String,
    #[arg(long)]
    next_release_prn: Option<String>,
    #[arg(long)]
    organization_prn: String,
    #[arg(long)]
    phase_value: f64,
    #[arg(long)]
    previous_release_prn: Option<String>,
    #[arg(long)]
    required: bool,
    #[arg(long)]
    schedule_date: String,
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
    #[arg(long)]
    pub limit: Option<u8>,
    #[arg(long)]
    pub order: Option<String>,
    #[arg(long)]
    pub search: String,
    #[arg(long)]
    pub page: Option<String>,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListReleasesParams {
            limit: self.inner.limit,
            order: self.inner.order,
            search: self.inner.search,
            page: self.inner.page,
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
    #[arg(long)]
    pub prn: String,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long)]
    pub name: Option<String>,

    #[arg(long)]
    pub phase_value: Option<f64>,

    #[arg(long)]
    pub required: Option<bool>,

    #[arg(long)]
    pub schedule_date: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateReleaseParams {
            prn: self.inner.prn,
            description: self.inner.description,
            name: self.inner.name,
            phase_value: self.inner.phase_value,
            required: self.inner.required,
            schedule_date: self.inner.schedule_date,
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
