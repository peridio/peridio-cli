use super::Command;
use crate::api::list::ListArgs;
use crate::print_json;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::cohorts::{
    CreateCohortParams, GetCohortParams, ListCohortsParams, UpdateCohortParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum CohortsCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
}

impl CohortsCommand {
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
    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    name: String,

    /// The PRN of the organization you wish to create the resource within.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Organization)
    )]
    organization_prn: String,

    /// The PRN of the product you wish to create the resource within.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Product)
    )]
    product_prn: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateCohortParams {
            description: self.inner.description,
            name: self.inner.name,
            organization_prn: self.inner.organization_prn,
            product_prn: self.inner.product_prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.cohorts().create(params).await.context(ApiSnafu)? {
            Some(cohort) => print_json!(&cohort),
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
        let params = ListCohortsParams {
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

        match api.cohorts().list(params).await.context(ApiSnafu)? {
            Some(cohort) => print_json!(&cohort),
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
        value_parser = PRNValueParser::new(PRNType::Cohort)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetCohortParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.cohorts().get(params).await.context(ApiSnafu)? {
            Some(cohort) => print_json!(&cohort),
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
        value_parser = PRNValueParser::new(PRNType::Cohort)
    )]
    prn: String,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    pub description: Option<String>,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    pub name: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateCohortParams {
            prn: self.inner.prn,
            description: self.inner.description,
            name: self.inner.name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.cohorts().update(params).await.context(ApiSnafu)? {
            Some(cohort) => print_json!(&cohort),
            None => panic!(),
        }

        Ok(())
    }
}
