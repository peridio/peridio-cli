mod identity;

use std::ops::Deref;

use snafu::ResultExt;
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

impl<T: StructOpt> Deref for Command<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(StructOpt, Debug)]
pub enum ApiCommand {
    /// Retrieve identity
    Identity(Command<identity::IdentityCommand>),
}

impl ApiCommand {
    pub(crate) async fn run(self) -> Result<(), crate::Error> {
        match self {
            ApiCommand::Identity(cmd) => identity::run(cmd).await.context(crate::ApiSnafu)?,
        };

        Ok(())
    }
}
