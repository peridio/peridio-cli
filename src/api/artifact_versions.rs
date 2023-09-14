use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::artifact_versions::{
    CreateArtifactVersionParams, GetArtifactVersionParams, ListArtifactVersionsParams,
    UpdateArtifactVersionParams,
};
use peridio_sdk::api::{Api, ApiOptions};
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum ArtifactVersionsCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
}

impl ArtifactVersionsCommand {
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
    artifact_prn: String,

    #[arg(long)]
    description: Option<String>,

    #[arg(long)]
    version: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateArtifactVersionParams {
            artifact_prn: self.inner.artifact_prn,
            description: self.inner.description,
            version: self.inner.version,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .artifact_versions()
            .create(params)
            .await
            .context(ApiSnafu)?
        {
            Some(artifact_version) => print_json!(&artifact_version),
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
        let params = ListArtifactVersionsParams {
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

        match api
            .artifact_versions()
            .list(params)
            .await
            .context(ApiSnafu)?
        {
            Some(artifact_version) => print_json!(&artifact_version),
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
        let params = GetArtifactVersionParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .artifact_versions()
            .get(params)
            .await
            .context(ApiSnafu)?
        {
            Some(artifact_version) => print_json!(&artifact_version),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    #[arg(long)]
    prn: String,

    #[arg(long)]
    pub description: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateArtifactVersionParams {
            prn: self.inner.prn,
            description: self.inner.description,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .artifact_versions()
            .update(params)
            .await
            .context(ApiSnafu)?
        {
            Some(artifact_version) => print_json!(&artifact_version),
            None => panic!(),
        }

        Ok(())
    }
}
