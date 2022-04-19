use super::binary::BinaryCommand;
use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::{Api, VersionChangeset};
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum VersionCommand {
    /// Create an version
    Create(Command<CreateCommand>),

    /// Get an version
    Get(Command<GetCommand>),

    /// List versions
    List(Command<ListCommand>),

    /// Operate on binaries
    Binary(BinaryCommand),
}

impl VersionCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Get(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
            Self::Binary(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    /// An element id
    #[structopt(long)]
    pub element_id: String,

    /// A version number string
    #[structopt(long)]
    number: String,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let version = VersionChangeset {
            number: self.inner.number.clone(),
        };

        let version = api
            .element(&self.inner.element_id)
            .versions()
            .create(version)
            .await
            .context(ApiSnafu)?;

        print_json!(&version);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    /// An element id
    #[structopt(long)]
    pub element_id: String,

    /// A version id
    #[structopt(long)]
    pub version_id: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let version = api
            .element(&self.inner.element_id)
            .version(&self.inner.version_id)
            .get()
            .await
            .context(ApiSnafu)?;

        print_json!(&version);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {
    /// An element id
    #[structopt(long)]
    pub element_id: String,
}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);
        let versions = api
            .element(&self.inner.element_id)
            .versions()
            .list()
            .await
            .context(ApiSnafu)?;

        print_json!(&versions);

        Ok(())
    }
}
