use super::Command;
use crate::api::CliCommands;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::Api;
use snafu::ResultExt;

#[derive(Parser, Debug)]
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

#[derive(Parser, Debug)]
pub struct MeCommand {}

impl Command<MeCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        // require api key
        let mut missing_arguments = Vec::new();

        if global_options.api_key.is_none() {
            missing_arguments.push("--api-key".to_owned());
        }

        CliCommands::print_missing_arguments(missing_arguments);

        let api = Api::from(global_options);

        match api.users().me().await.context(ApiSnafu)? {
            Some(users_me) => print_json!(&users_me),
            None => panic!(),
        }

        Ok(())
    }
}
