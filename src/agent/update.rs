use super::Command;
use peridio_sdk::node_client::NodeClient;
use peridio_sdk::NodeUpdateRequest;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Update {
    #[structopt(long)]
    pub base_url: String,
}

pub(super) async fn run(cmd: Command<Update>) {
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
    let request = tonic::Request::new(NodeUpdateRequest {
        base_url: (*cmd.base_url).to_string(),
    });

    match client.update(request).await {
        Ok(response) => println!("{:?}", response.into_inner()),
        Err(e) => println!("{}", e),
    }
}
