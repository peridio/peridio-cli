use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use clap::Parser;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
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

#[derive(Parser, Debug)]
pub struct MeCommand {}

impl Command<MeCommand> {
    async fn run(self) -> Result<(), Error> {
        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api.users().me().await.context(ApiSnafu)? {
            Some(users_me) => print_json!(&users_me),
            None => panic!(),
        }

        Ok(())
    }
}
