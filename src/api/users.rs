use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum UsersCommand {
    Me(Command<MeCommand>),
}

impl UsersCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Me(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct MeCommand {}

impl Command<MeCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
        });

        match api.users().me().await.context(ApiSnafu)? {
            Some(users_me) => print_json!(&users_me),
            None => panic!(),
        }

        Ok(())
    }
}
