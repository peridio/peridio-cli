use super::{util, Command};
use peridio_sdk::agent::Error;
use peridio_sdk::NodeStateRequest;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct State {}

pub(super) async fn run(cmd: Command<State>) -> Result<(), Error> {
    let mut client = util::create_client(&cmd).await?;
    let request = tonic::Request::new(NodeStateRequest {});

    let response = client.state(request).await?;
    println!("{:?}", response.into_inner());

    Ok(())
}
