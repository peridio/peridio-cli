mod api;
mod config;
mod utils;

use std::process::ExitCode;
use std::{
    fmt,
    io::{self, ErrorKind},
    path::{self, PathBuf},
};

use clap::Parser;
use config::Config;
use snafu::Snafu;

use crate::config::config_v2::{CertificateAuthoritiesV2, SigningKeyPairsV2};
use crate::utils::{Style, StyledStr};

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

    #[snafu(display("{:?}", path))]
    NonExistingPath {
        path: path::PathBuf,
        source: io::Error,
    },

    #[snafu(display("Failed to create certificate parameters: {}", source))]
    CertParamsCreation { source: rcgen::Error },

    #[snafu(display("Failed to create certificate: {}", source))]
    CertificateCreation { source: rcgen::Error },

    #[snafu(display("Failed to parse date: {}", source))]
    DateParse { source: time::error::Parse },
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Parser)]
#[command(name = "peridio", version = env!("PERIDIO_CLI_VERSION"))]
struct Program {
    #[command(flatten)]
    global_options: GlobalOptions,

    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Clone, Debug)]
pub struct GlobalOptions {
    #[arg(long, env = "PERIDIO_API_KEY", hide_env_values = true, short = 'a')]
    api_key: Option<String>,

    #[arg(long, env = "PERIDIO_BASE_URL", short = 'b')]
    base_url: Option<String>,

    #[arg(long, env = "PERIDIO_CA_PATH", short = 'c')]
    ca_path: Option<PathBuf>,

    #[arg(long, env = "PERIDIO_ORGANIZATION_NAME", short = 'o')]
    organization_name: Option<String>,

    #[arg(long, env = "PERIDIO_PROFILE", short = 'p')]
    profile: Option<String>,

    #[arg(
        long,
        env = "PERIDIO_CONFIG_DIRECTORY",
        short = 'd',
        requires = "profile"
    )]
    config_directory: Option<String>,

    #[clap(skip)]
    signing_key_pairs: Option<SigningKeyPairsV2>,

    #[clap(skip)]
    certificate_authorities: Option<CertificateAuthoritiesV2>,
}

impl Program {
    async fn run(mut self) -> Result<(), Error> {
        if let Some(path) = &self.global_options.ca_path {
            if !path.exists() {
                return Err(Error::NonExistingPath {
                    path: path.to_path_buf(),
                    source: std::io::Error::from(ErrorKind::NotFound),
                });
            }
        }

        // parse config files if profile config is provided

        match self.command {
            Command::CliCommand(api::CliCommands::Config(_)) => (),
            _ => {
                if let Some(config) = Config::parse(&self.global_options.config_directory) {
                    if let Some(profile_name) = &self.global_options.profile {
                        if let Ok(profile) = Config::get_profile(&config, profile_name) {
                            // profile was provided
                            if self.global_options.api_key.is_none() {
                                if let Some(api_key) = profile.api_key {
                                    self.global_options.api_key = Some(api_key);
                                };
                            }

                            if self.global_options.base_url.is_none() {
                                if let Some(base_url) = profile.base_url {
                                    self.global_options.base_url = Some(base_url);
                                };
                            };

                            if self.global_options.ca_path.is_none() {
                                if let Some(ca_path) = profile.ca_path {
                                    self.global_options.ca_path = Some(ca_path.into());
                                };
                            };

                            if self.global_options.organization_name.is_none() {
                                if let Some(organization_name) = profile.organization_name {
                                    self.global_options.organization_name = Some(organization_name);
                                };
                            }
                        }
                    }

                    self.global_options.signing_key_pairs = config.signing_key_pairs;
                    self.global_options.certificate_authorities = config.certificate_authorities;
                }
            }
        };

        match self.command {
            Command::CliCommand(cmd) => cmd.run(self.global_options).await?,
        };

        Ok(())
    }
}

#[derive(Parser)]
#[command(about = "Work with Peridio from the command line.")]
enum Command {
    #[command(flatten)]
    CliCommand(api::CliCommands),
}

#[tokio::main]
async fn main() -> ExitCode {
    match Program::parse().run().await {
        Err(error) => {
            match error {
                Error::Api { source } => {
                    eprintln!("{source}")
                }

                Error::NonExistingPath { path, source: _ } => {
                    let mut error = StyledStr::new();
                    error.push_str(Some(Style::Error), "error: ".to_string());
                    error.push_str(None, "Path does not exist:\r\n".to_string());
                    error.push_str(Some(Style::Warning), format!("\t{}", path.display()));
                    error.print_data_err();
                }

                error => eprintln!("Error: {error}"),
            }

            ExitCode::FAILURE
        }
        Ok(()) => ExitCode::SUCCESS,
    }
}
