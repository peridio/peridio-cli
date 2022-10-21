mod api;

use std::{
    fmt,
    io::{self, ErrorKind},
    path::{self, PathBuf},
};

use clap::Parser;
use snafu::Snafu;

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

    #[snafu(display("{:?}", path))]
    NonExistingPath {
        path: path::PathBuf,
        source: io::Error,
    },
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Parser)]
#[command(name = "peridio", version = env!("MOREL_VERSION"))]
struct Program {
    #[command(flatten)]
    global_options: GlobalOptions,

    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
pub struct GlobalOptions {
    #[arg(long, env = "PERIDIO_API_KEY", hide_env_values = true, global = true)]
    api_key: Option<String>,

    #[arg(long, env = "PERIDIO_BASE_URL", global = true)]
    base_url: Option<String>,

    #[arg(long, env = "PERIDIO_CA_PATH", global = true)]
    ca_path: Option<PathBuf>,

    #[arg(long, env = "PERIDIO_ORGANIZATION_NAME", global = true)]
    organization_name: Option<String>,
}

impl Program {
    async fn run(self) -> Result<(), Error> {
        if let Some(path) = self.global_options.ca_path {
            if !path.exists() {
                return Err(Error::NonExistingPath {
                    path,
                    source: std::io::Error::from(ErrorKind::NotFound),
                });
            }
        }

        match self.command {
            Command::Api(cmd) => cmd.run().await?,
        };

        Ok(())
    }
}

#[derive(Parser)]
#[command(about = "Work with Peridio from the command line.")]
enum Command {
    #[command(flatten)]
    Api(api::ApiCommand),
}

#[tokio::main]
async fn main() {
    if let Err(e) = Program::parse().run().await {
        match e {
            Error::Api { source } => {
                eprintln!("{}", source)
            }

            Error::NonExistingPath { path, source: _ } => {
                use std::io::Write;
                use termcolor::WriteColor;

                let bufwtr = termcolor::BufferWriter::stderr(termcolor::ColorChoice::Always);
                let mut buffer = bufwtr.buffer();

                buffer
                    .set_color(
                        termcolor::ColorSpec::new()
                            .set_fg(Some(termcolor::Color::Red))
                            .set_bold(true),
                    )
                    .unwrap();

                write!(&mut buffer, "error: ").unwrap();

                buffer.set_color(&termcolor::ColorSpec::new()).unwrap();

                writeln!(&mut buffer, "Path does not exist:").unwrap();

                buffer
                    .set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Yellow)))
                    .unwrap();

                writeln!(&mut buffer, "\t{}", path.display()).unwrap();

                bufwtr.print(&buffer).unwrap();

                // DATAERR
                std::process::exit(65);
            }

            error => eprintln!("Error: {}", error),
        }
    }
}
