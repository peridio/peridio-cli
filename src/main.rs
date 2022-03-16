mod agent;
mod api;

use structopt::StructOpt;

#[derive(StructOpt)]
struct Program {
    #[structopt(subcommand)]
    command: Command,
}

impl Program {
    async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Program::from_args().run().await
}
