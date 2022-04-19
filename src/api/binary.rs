use std::path::PathBuf;

use super::Command;
use crate::{print_json, ApiSnafu, Error, FileSnafu};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;
use tokio::fs::File;
use tokio::io::{self, AsyncRead};

#[derive(StructOpt, Debug)]
pub enum BinaryCommand {
    /// Create a binary
    Create(Command<CreateCommand>),
}

impl BinaryCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    /// A path to a local binary.
    /// If no file is provided, reads from standard input
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

        let rdr: Box<dyn AsyncRead + Sync + Send + Unpin> = match &self.inner.file {
            Some(v) => Box::new(File::open(v).await.context(FileSnafu)?),
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
