use std::path::PathBuf;

use super::Command;
use crate::{print_json, ApiSnafu, Error, FileMetadataSnafu, FileSnafu};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;
use tokio::fs::File;
use tokio::io::{self, AsyncRead};

#[derive(StructOpt, Debug)]
pub enum BinaryCommand {
    /// Create a binary
    Create(Command<CreateCommand>),

    /// Get a binary
    Get(Command<GetCommand>),

    /// List binaries
    List(Command<ListCommand>),
}

impl BinaryCommand {
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
    /// A path to a local binary.
    /// If no file is provided, then the standard input is read.
    #[structopt(long, parse(from_os_str))]
    pub file: Option<PathBuf>,

    /// An element id
    #[structopt(long)]
    pub element_id: String,

    /// A version id
    #[structopt(long)]
    pub version_id: String,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);

        let rdr: Box<dyn AsyncRead + Sync + Send + Unpin> = match self.inner.file {
            Some(path) => Box::new(validate_file(&path).await?),
            None => Box::new(io::stdin()),
        };

        let binary = api
            .element(&self.inner.element_id)
            .version(&self.inner.version_id)
            .binaries()
            .create(rdr)
            .await
            .context(ApiSnafu)?;

        print_json!(&binary);

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

    /// A binary id
    #[structopt(long)]
    pub binary_id: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);

        let binary = api
            .element(&self.inner.element_id)
            .version(&self.inner.version_id)
            .binary(&self.inner.binary_id)
            .get()
            .await
            .context(ApiSnafu)?;

        print_json!(&binary);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {
    /// An element id
    #[structopt(long)]
    pub element_id: String,

    /// A version id
    #[structopt(long)]
    pub version_id: String,
}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);

        let binaries = api
            .element(&self.inner.element_id)
            .version(&self.inner.version_id)
            .binaries()
            .list()
            .await
            .context(ApiSnafu)?;

        print_json!(&binaries);

        Ok(())
    }
}

async fn validate_file(path: &PathBuf) -> Result<File, Error> {
    let file = File::open(path).await.context(FileSnafu)?;

    let metadata = file.metadata().await.context(FileMetadataSnafu)?;

    match metadata.len() {
        0 => Err(Error::EmptyFile {
            path: path.to_owned(),
        }),
        _ => Ok(file),
    }
}
