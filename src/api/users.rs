use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum UsersCommand {
    Me(Command<MeCommand>),
}

impl UsersCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Me(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct MeCommand {}

impl Command<MeCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);

        match api.users().me().await.context(ApiSnafu)? {
            Some(users_me) => print_json!(&users_me),
            None => panic!(),
        }

        Ok(())
    }
}
