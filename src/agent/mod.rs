mod state;
mod update;
mod util;

use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;

use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Command<T: StructOpt> {
    #[structopt(long, parse(from_os_str), required_unless = "socket-addr")]
    pub socket_path: Option<PathBuf>,

    /// 127.0.0.1:8080 or [::1]:8080
    #[structopt(long, parse(try_from_str), required_unless = "socket-path")]
    pub socket_addr: Option<SocketAddr>,

    #[structopt(flatten)]
    inner: T,
}

impl<T: StructOpt> Deref for Command<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(StructOpt)]
pub enum AgentCommand {
    /// Retrieve node state
    State(Command<state::State>),
    /// Update node
    Update(Command<update::Update>),
}

impl AgentCommand {
    pub(crate) async fn run(self) -> Result<(), crate::Error> {
        let result = match self {
            Self::State(cmd) => state::run(cmd).await,
            Self::Update(cmd) => update::run(cmd).await,
        };

        result.context(crate::AgentSnafu)?;

        Ok(())
    }
}
