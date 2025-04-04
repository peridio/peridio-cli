use std::fs;

use super::Command;
use crate::api::list::ListArgs;
use crate::print_json;
use crate::utils::maybe_json;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use crate::NonExistingPathSnafu;
use clap::Parser;
use peridio_sdk::api::artifacts::{
    CreateArtifactParams, GetArtifactParams, ListArtifactsParams, UpdateArtifactParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum ArtifactsCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
}

impl ArtifactsCommand {
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
    /// A JSON object that informs the metadata that will be associated with this artifact's binaries when they are included in bundles.
    #[arg(long, conflicts_with = "custom_metadata_path")]
    custom_metadata: Option<String>,

    /// The path to the JSON file value for custom_metadata
    #[arg(long, conflicts_with = "custom_metadata")]
    custom_metadata_path: Option<String>,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,

    /// A user provided custom UUID id for the artifact database record.
    #[arg(long)]
    id: Option<String>,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    name: String,

    /// The PRN of the organization you wish to create the resource within.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Organization)
    )]
    organization_prn: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let custom_metadata = if let Some(custom_metadata_path) = self.inner.custom_metadata_path {
            fs::read_to_string(&custom_metadata_path)
                .context(NonExistingPathSnafu {
                    path: &custom_metadata_path,
                })?
                .into()
        } else {
            self.inner.custom_metadata
        };

        let params = CreateArtifactParams {
            custom_metadata: maybe_json(custom_metadata),
            description: self.inner.description,
            id: self.inner.id,
            name: self.inner.name,
            organization_prn: self.inner.organization_prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.artifacts().create(params).await.context(ApiSnafu)? {
            Some(artifact) => print_json!(&artifact),
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
        let params = ListArtifactsParams {
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

        match api.artifacts().list(params).await.context(ApiSnafu)? {
            Some(artifact) => print_json!(&artifact),
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
        value_parser = PRNValueParser::new(PRNType::Artifact)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetArtifactParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.artifacts().get(params).await.context(ApiSnafu)? {
            Some(artifact) => print_json!(&artifact),
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
        value_parser = PRNValueParser::new(PRNType::Artifact)
    )]
    prn: String,

    /// A JSON object that informs the metadata that will be associated with this artifact's binaries when they are included in bundles.
    #[arg(long)]
    pub custom_metadata: Option<String>,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    pub description: Option<String>,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    pub name: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateArtifactParams {
            prn: self.inner.prn,
            custom_metadata: maybe_json(self.inner.custom_metadata),
            description: self.inner.description,
            name: self.inner.name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.artifacts().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
