mod agent;
mod api;

use std::{fmt, io, path};

use snafu::Snafu;
use structopt::StructOpt;

#[macro_export]
macro_rules! print_json {
    ($v:expr) => {
        println!(
            "{}",
            serde_json::to_string($v).context(crate::JsonSerializationSnafu)?
        );
    };
}

#[derive(Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Agent error {}", source))]
    Agent { source: peridio_sdk::agent::Error },

    #[snafu(display("Api error {}", source))]
    Api { source: peridio_sdk::api::Error },

    #[snafu(display("Unable to serialize to JSON {}", source))]
    JsonSerialization { source: serde_json::Error },

    #[snafu(display("Unable to open file {}", source))]
    File { source: io::Error },

    #[snafu(display("File {:?} is empty", path))]
    EmptyFile { path: path::PathBuf },

    #[snafu(display("Unable to retrieve file metadata {:?}", source))]
    FileMetadata { source: io::Error },
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(StructOpt)]
#[structopt(version = env!("MOREL_VERSION"))]
struct Program {
    #[structopt(subcommand)]
    command: Command,
}

impl Program {
    async fn run(self) -> Result<(), Error> {
        match self.command {
            Command::Api(cmd) => cmd.run().await?,
            Command::Node(cmd) => cmd.run().await?,
        };

        Ok(())
    }
}

#[derive(StructOpt)]
#[structopt(about = "interact with local or network connected nodes")]
enum Command {
    #[structopt(flatten)]
    Api(api::ApiCommand),

    /// Interact with local or network connected nodes
    Node(agent::AgentCommand),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    Program::from_args().run().await
}
