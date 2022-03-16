mod state;
mod update;

use std::net::IpAddr;
use std::ops::Deref;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Command<T: StructOpt> {
    #[structopt(long, parse(from_os_str), required_unless = "socket-addr")]
    pub socket_path: Option<PathBuf>,

    #[structopt(long, required_unless = "socket-path", requires = "socket-port")]
    pub socket_addr: Option<IpAddr>,

    #[structopt(long, requires = "socket-addr")]
    pub socket_port: Option<u16>,

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
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Self::State(cmd) => state::run(cmd).await,
            Self::Update(cmd) => update::run(cmd).await,
        };

        Ok(())
    }
}
