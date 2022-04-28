use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::{Api, NodeTypeChangeset};
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum NodeTypeCommand {
    /// Create a node-type
    Create(Command<CreateCommand>),

    /// Get a node-type
    Get(Command<GetCommand>),

    /// Update a node-type
    Update(Command<UpdateCommand>),

    /// List node-types
    List(Command<ListCommand>),
}

impl NodeTypeCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Get(cmd) => cmd.run().await,
            Self::Update(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    /// A node-type name
    #[structopt(long)]
    name: String,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let node_type = NodeTypeChangeset {
            name: self.inner.name,
        };

        let node_type = api.node_types().create(node_type).await.context(ApiSnafu)?;

        print_json!(&node_type);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    /// A node-type id
    #[structopt(long)]
    id: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let node_type = api
            .node_type(&self.inner.id)
            .get()
            .await
            .context(ApiSnafu)?;

        print_json!(&node_type);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct UpdateCommand {
    /// An element id
    #[structopt(long)]
    id: String,

    /// An element name
    #[structopt(long)]
    name: String,
}

impl Command<UpdateCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let changeset = NodeTypeChangeset {
            name: self.inner.name,
        };

        let node_type = api
            .node_type(&self.inner.id)
            .update(changeset)
            .await
            .context(ApiSnafu)?;

        print_json!(&node_type);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let node_types = api.node_types().list().await.context(ApiSnafu)?;

        print_json!(&node_types);

        Ok(())
    }
}
