use super::Command;
use peridio_sdk::{api::Error, Api};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct IdentityCommand {}

pub(super) async fn run(cmd: Command<IdentityCommand>) -> Result<(), Error> {
    let api = Api::new(cmd.api_key, cmd.base_url);
    let identity = api.identity().await?;
    println!("{:?}", identity);

    Ok(())
}
