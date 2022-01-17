use crate::cli::{NodeCommand, NodeOpts, UpdateOpts};
use peridio_sdk::node_client::NodeClient;
use peridio_sdk::{NodeStateRequest, NodeUpdateRequest};

pub async fn handle_command(opts: &NodeOpts) {
    match &opts.node_command {
        NodeCommand::State(_cfg) => {
            get_state(opts).await;
        }
        NodeCommand::Update(cfg) => {
            update(opts, cfg).await;
        }
    }
}

async fn get_state(opts: &NodeOpts) {
    let channel =
        match peridio_sdk::channel(opts.socket_path.clone(), opts.socket_addr, opts.socket_port)
            .await
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

async fn update(opts: &NodeOpts, update_opts: &UpdateOpts) {
    let channel =
        match peridio_sdk::channel(opts.socket_path.clone(), opts.socket_addr, opts.socket_port)
            .await
        {
            Err(e) => {
                println!("Error connecting to node\n{}", e);
                return;
            }
            Ok(channel) => channel,
        };

    let mut client = NodeClient::new(channel);
    let request = tonic::Request::new(NodeUpdateRequest {
        base_url: update_opts.base_url.clone(),
    });

    match client.update(request).await {
        Ok(response) => println!("{:?}", response.into_inner()),
        Err(e) => println!("{}", e),
    }
}
