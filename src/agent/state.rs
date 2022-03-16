use super::Command;
use peridio_sdk::node_client::NodeClient;
use peridio_sdk::NodeStateRequest;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct State {}

pub(super) async fn run(cmd: Command<State>) {
    let channel =
        match peridio_sdk::channel(cmd.socket_path.clone(), cmd.socket_addr, cmd.socket_port).await
        {
            Err(e) => {
                println!("Error connecting to node\n{}", e);
                return;
            }
            Ok(channel) => channel,
        };

    let mut client = NodeClient::new(channel);
    let request = tonic::Request::new(NodeStateRequest {});

    match client.state(request).await {
        Ok(response) => println!("{}", response.into_inner()),
        Err(e) => println!("{}", e),
    }
}
