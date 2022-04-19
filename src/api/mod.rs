mod binary;
mod element;
mod identity;
mod version;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Command<T: StructOpt> {
    #[structopt(long, env = "PERIDIO_API_KEY")]
    pub api_key: String,

    #[structopt(long, env = "PERIDIO_BASE_URL")]
    pub base_url: Option<String>,

    #[structopt(flatten)]
    inner: T,
}

#[derive(StructOpt, Debug)]
pub enum ApiCommand {
    /// Retrieve identity
    Identity(Command<identity::IdentityCommand>),

    /// Operate on elements
    Element(element::ElementCommand),
}

impl ApiCommand {
    pub(crate) async fn run(self) -> Result<(), crate::Error> {
        match self {
            ApiCommand::Identity(cmd) => identity::run(cmd).await?,
            ApiCommand::Element(cmd) => cmd.run().await?,
        };

        Ok(())
    }
}
