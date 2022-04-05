mod agent;
mod api;

use std::fmt;

use snafu::Snafu;
use structopt::StructOpt;

#[derive(Snafu)]
#[snafu(visibility(pub(crate)))]
enum Error {
    #[snafu(display("Agent error {}", source))]
    Agent { source: peridio_sdk::agent::Error },

    #[snafu(display("Api error {}", source))]
    Api { source: peridio_sdk::api::Error },
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(StructOpt)]
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
