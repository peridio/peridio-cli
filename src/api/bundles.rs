use super::Command;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::bundles::{
    CreateBundleParams, DeleteBundleParams, GetBundleParams, ListBundlesParams, UpdateBundleParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum BundlesCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
    Delete(Command<DeleteCommand>),
}

impl BundlesCommand {
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
    /// The PRNs of the artifact versions to include binaries from in the bundle.
    ///
    /// Values can be provided by passing each value in a flag
    /// or by delimiting all values with ","
    #[clap(short, long, value_parser, num_args = 1.., value_delimiter = ',')]
    artifact_version_prns: Vec<String>,

    /// A user provided custom UUID id for the bundle database record.
    #[arg(long)]
    id: Option<String>,

    /// The name of the bundle.
    #[arg(long)]
    name: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateBundleParams {
            artifact_version_prns: self.inner.artifact_version_prns,
            id: self.inner.id,
            name: self.inner.name,
        };

        let api = Api::from(global_options);

        match api.bundles().create(params).await.context(ApiSnafu)? {
            Some(bundle) => print_json!(&bundle),
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
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteBundleParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        api.bundles().delete(params).await.context(ApiSnafu)?;

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
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

        match api.bundles().list(params).await.context(ApiSnafu)? {
            Some(bundle) => print_json!(&bundle),
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
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetBundleParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        match api.bundles().get(params).await.context(ApiSnafu)? {
            Some(bundle) => print_json!(&bundle),
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
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    prn: String,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    pub name: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateBundleParams {
            prn: self.inner.prn,
            name: self.inner.name,
        };

        let api = Api::from(global_options);

        match api.bundles().update(params).await.context(ApiSnafu)? {
            Some(response) => print_json!(&response),
            None => panic!(),
        }

        Ok(())
    }
}
