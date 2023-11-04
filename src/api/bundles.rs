use super::Command;
use crate::api::list::ListArgs;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::bundles::{CreateBundleParams, GetBundleParams, ListBundlesParams};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum BundlesCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
}

impl BundlesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]

pub struct CreateCommand {
    #[clap(short, long, value_parser, num_args = 1.., value_delimiter = ' ')]
    artifact_version_prns: Vec<String>,

    #[arg(long)]
    organization_prn: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateBundleParams {
            artifact_version_prns: self.inner.artifact_version_prns,
            organization_prn: self.inner.organization_prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.bundles().create(params).await.context(ApiSnafu)? {
            Some(bundle) => print_json!(&bundle),
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
        let params = ListBundlesParams {
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

        match api.bundles().list(params).await.context(ApiSnafu)? {
            Some(bundle) => print_json!(&bundle),
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
        let params = GetBundleParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.bundles().get(params).await.context(ApiSnafu)? {
            Some(bundle) => print_json!(&bundle),
            None => panic!(),
        }

        Ok(())
    }
}
