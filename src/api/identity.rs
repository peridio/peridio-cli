use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::Api;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct IdentityCommand {}

pub(super) async fn run(cmd: Command<IdentityCommand>) -> Result<(), Error> {
    let api = Api::new(cmd.api_key, cmd.base_url);
    let identity = api.identity().await.context(ApiSnafu)?;

    print_json!(&identity);

    Ok(())
}
