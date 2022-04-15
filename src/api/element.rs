use super::version::VersionCommand;
use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::{Api, ElementChangeset};
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum ElementCommand {
    /// Create an element
    Create(Command<CreateCommand>),

    /// Update an element
    Update(Command<UpdateCommand>),

    /// List elements
    List(Command<ListCommand>),

    /// Get an element
    Get(Command<GetCommand>),

    /// Operate on versions
    Version(VersionCommand),
}

impl ElementCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Update(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
            Self::Get(cmd) => cmd.run().await,
            Self::Version(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    /// An element name
    #[structopt(long)]
    name: String,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let element = ElementChangeset {
            name: self.inner.name,
        };

        let element = api.elements().create(element).await.context(ApiSnafu)?;

        print_json!(&element);

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
        let changeset = ElementChangeset {
            name: self.inner.name,
        };

        let element = api
            .element(&self.inner.id)
            .update(changeset)
            .await
            .context(ApiSnafu)?;

        print_json!(&element);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let elements = api.elements().list().await.context(ApiSnafu)?;

        print_json!(&elements);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    /// An element id
    #[structopt(long)]
    id: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let element = api.element(&self.inner.id).get().await.context(ApiSnafu)?;

        print_json!(&element);

        Ok(())
    }
}
