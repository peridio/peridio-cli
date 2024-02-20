use super::Command;
use crate::print_json;
use crate::utils::{Style, StyledStr};
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
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
        let mut error_vec = Vec::new();

        if global_options.api_key.is_none() {
            error_vec.push("--api-key".to_owned());
        }

        if !error_vec.is_empty() {
            let mut error = StyledStr::new();

            error.push_str(Some(Style::Error), "error: ".to_string());
            error.push_str(
                None,
                "The following arguments are required at the global level:\r\n".to_string(),
            );
            for error_msg in error_vec.iter() {
                error.push_str(Some(Style::Success), format!("\t{error_msg}\r\n"));
            }
            error.print_data_err();
        }

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.users().me().await.context(ApiSnafu)? {
            Some(users_me) => print_json!(&users_me),
            None => panic!(),
        }

        Ok(())
    }
}
