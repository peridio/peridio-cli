use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::{Api, DistributionChangeset};
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum DistributionCommand {
    /// Create a distribution
    Create(Command<CreateCommand>),

    /// Get a distribution
    Get(Command<GetCommand>),

    /// List distributions
    List(Command<ListCommand>),
}

impl DistributionCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Get(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    /// A distribution name
    #[structopt(long)]
    name: String,

    /// The associated element version id
    #[structopt(long)]
    element_version_id: String,

    /// A parent distribution id
    #[structopt(long)]
    next_distribution_id: Option<String>,

    /// A node group id
    #[structopt(long)]
    node_group_id: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let distribution = DistributionChangeset {
            name: self.inner.name,
            element_version_id: self.inner.element_version_id,
            next_distribution_id: self.inner.next_distribution_id,
            node_group_id: self.inner.node_group_id,
        };

        let distribution = api
            .distributions()
            .create(distribution)
            .await
            .context(ApiSnafu)?;

        print_json!(&distribution);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    /// A distribution id
    #[structopt(long)]
    id: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let distribution = api
            .distribution(&self.inner.id)
            .get()
            .await
            .context(ApiSnafu)?;

        print_json!(&distribution);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let distributions = api.distributions().list().await.context(ApiSnafu)?;

        print_json!(&distributions);

        Ok(())
    }
}
