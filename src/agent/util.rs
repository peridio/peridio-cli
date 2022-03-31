use super::Command;
use peridio_sdk::agent::{self, Error};
use peridio_sdk::node_client::NodeClient;
use structopt::StructOpt;
use tonic::transport::Channel;

pub(super) async fn create_client<T: StructOpt>(
    cmd: &Command<T>,
) -> Result<NodeClient<Channel>, Error> {
    let address = match (&cmd.socket_path, &cmd.socket_addr) {
        (Some(v), None) => v.to_owned().into(),
        (None, Some(v)) => v.to_owned().into(),
        _ => unreachable!("socket_addr or socket_path should be defined"),
    };

    let channel = agent::channel(address).await?;

    Ok(NodeClient::new(channel))
}
