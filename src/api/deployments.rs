use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::deployments::CreateDeploymentParams;
use peridio_sdk::api::deployments::DeleteDeploymentParams;
use peridio_sdk::api::deployments::DeploymentCondition;
use peridio_sdk::api::deployments::GetDeploymentParams;
use peridio_sdk::api::deployments::ListDeploymentParams;
use peridio_sdk::api::deployments::UpdateDeployment;
use peridio_sdk::api::deployments::UpdateDeploymentCondition;
use peridio_sdk::api::deployments::UpdateDeploymentParams;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;
use uuid::Uuid;

#[derive(Parser, Debug)]
pub enum DeploymentsCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
}

impl DeploymentsCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    #[arg(long)]
    delta_updatable: bool,

    #[arg(long)]
    firmware: Uuid,

    #[arg(long)]
    product_name: String,

    #[arg(long)]
    name: String,

    #[arg(long, required = true)]
    tags: Vec<String>,

    #[arg(long)]
    version: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateDeploymentParams {
            firmware: self.inner.firmware.to_string(),
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            name: self.inner.name,
            is_active: false, // must be false
            conditions: &DeploymentCondition {
                tags: self.inner.tags,
                version: self.inner.version,
            },
            delta_updatable: self.inner.delta_updatable,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.deployments().create(params).await.context(ApiSnafu)? {
            Some(deployment) => print_json!(&deployment),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    #[arg(long)]
    deployment_name: String,

    #[arg(long)]
    product_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteDeploymentParams {
            deployment_name: self.inner.deployment_name,
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        if (api.deployments().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    deployment_name: String,

    #[arg(long)]
    product_name: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetDeploymentParams {
            deployment_name: self.inner.deployment_name,
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.deployments().get(params).await.context(ApiSnafu)? {
            Some(deployment) => print_json!(&deployment),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[arg(long)]
    product_name: String,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListDeploymentParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.deployments().list(params).await.context(ApiSnafu)? {
            Some(deployments) => print_json!(&deployments),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    #[arg(long)]
    delta_updatable: Option<bool>,

    #[arg(long)]
    deployment_name: String,

    #[arg(long)]
    firmware: Option<Uuid>,

    #[arg(long)]
    is_active: Option<bool>,

    #[arg(long)]
    product_name: String,

    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    tags: Option<Vec<String>>,

    #[arg(long)]
    version: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let firmware = self.inner.firmware.map(|uuid| uuid.to_string());

        let mut condition = UpdateDeploymentCondition {
            tags: None,
            version: None,
        };

        let deployment = UpdateDeployment {
            name: self.inner.name,
            conditions: if self.inner.tags.is_some() || self.inner.version.is_some() {
                let tags = self.inner.tags;
                let version = self.inner.version;

                condition.tags = tags;
                condition.version = version;

                Some(&condition)
            } else {
                None
            },
            firmware,
            is_active: self.inner.is_active,
            delta_updatable: self.inner.delta_updatable,
        };

        let params = UpdateDeploymentParams {
            deployment_name: self.inner.deployment_name.to_string(),
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            deployment,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.deployments().update(params).await.context(ApiSnafu)? {
            Some(deployment) => print_json!(&deployment),
            None => panic!(),
        }

        Ok(())
    }
}
