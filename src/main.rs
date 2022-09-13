mod api;

use std::{fmt, io, path};

use snafu::Snafu;
use structopt::StructOpt;

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! print_json {
    ($v:expr) => {
        println!(
            "{}",
            serde_json::to_string($v).context(crate::JsonSerializationSnafu)?
        )
    };
}

#[derive(Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("{}", source))]
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
#[structopt(name = "peridio", version = env!("MOREL_VERSION"))]
struct Program {
    #[structopt(subcommand)]
    command: Command,
}

impl Program {
    async fn run(self) -> Result<(), Error> {
        match self.command {
            Command::Api(cmd) => cmd.run().await?,
        };

        Ok(())
    }
}

#[derive(StructOpt)]
#[structopt(about = "Work with Peridio from the command line.")]
enum Command {
    #[structopt(flatten)]
    Api(api::ApiCommand),
}

#[tokio::main]
async fn main() {
    if let Err(e) = Program::from_args().run().await {
        match e {
            Error::Api { source } => eprintln!("{}", source),

            error => eprintln!("Error: {}", error),
        }
    }
}
