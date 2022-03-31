use super::{util, Command};
use peridio_sdk::agent::Error;
use peridio_sdk::NodeUpdateRequest;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Update {
    #[structopt(long)]
    pub base_url: String,
}

pub(super) async fn run(cmd: Command<Update>) -> Result<(), Error> {
    let mut client = util::create_client(&cmd).await?;
    let request = tonic::Request::new(NodeUpdateRequest {
        base_url: (*cmd.base_url).to_string(),
    });

    let response = client.update(request).await?;
    println!("{:?}", response.into_inner());

    Ok(())
}
