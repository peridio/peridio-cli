use crate::node;
use std::net::IpAddr;
use std::path::PathBuf;
use structopt::StructOpt;

static ABOUT: &str = "
Peridio CLI.
";
static USAGE: &str = "peridio-cli [SUBCOMMAND] [FLAGS]";

#[derive(StructOpt, Debug)]
#[structopt(
    about = ABOUT,
    usage = USAGE,
)]
pub struct Opt {
    #[structopt(subcommand)]
    command: Option<Command>,
}

impl Opt {
    pub fn from_args() -> Opt {
        <Opt as StructOpt>::from_args()
    }

    pub async fn handle_command(self: Opt) {
        if let Some(subcommand) = &self.command {
            match subcommand {
                Command::Node(cfg) => {
                    node::handle_command(cfg).await;
                }
            }
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "command",
    about = "interact with local or network connected nodes"
)]
pub enum Command {
    #[structopt(name = "node")]
    Node(NodeOpts),
}

#[derive(StructOpt, Debug)]
pub struct NodeOpts {
    #[structopt(
        long = "socket-path",
        parse(from_os_str),
        required_unless = "socket-addr"
    )]
    pub socket_path: Option<PathBuf>,

    #[structopt(
        long = "socket-addr",
        required_unless = "socket-path",
        requires = "socket-port"
    )]
    pub socket_addr: Option<IpAddr>,

    #[structopt(long = "socket-port", requires = "socket-addr")]
    pub socket_port: Option<u16>,

    #[structopt(subcommand)]
    pub node_command: NodeCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "node-command", about = "node commands")]
pub enum NodeCommand {
    #[structopt(name = "state")]
    State(StateOpts),

    #[structopt(name = "update")]
    Update(UpdateOpts),
}

#[derive(StructOpt, Debug)]
pub struct StateOpts {}

#[derive(StructOpt, Debug)]
pub struct UpdateOpts {
    #[structopt(long = "base-url")]
    pub base_url: String,
}
