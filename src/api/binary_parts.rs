use std::fs;
use std::path::PathBuf;

use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::binary_parts::CreateBinaryPartParams;
use peridio_sdk::api::binary_parts::CreateBinaryPartResponse;
use peridio_sdk::api::binary_parts::ListBinaryPartsParams;
use peridio_sdk::api::binary_parts::ListBinaryPartsResponse;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum BinaryPartsCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
}

impl BinaryPartsCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]

pub struct CreateCommand {
    #[arg(long)]
    pub binary_prn: String,
    #[arg(
        long,
        conflicts_with("binary_content_path"),
        required_unless_present("binary_content_path")
    )]
    pub expected_binary_size: Option<u64>,
    #[arg(long)]
    pub hash: String,
    #[arg(long)]
    pub index: u16,
    #[arg(long)]
    pub size: u64,
    #[arg(
        long,
        conflicts_with("expected_binary_size"),
        required_unless_present("expected_binary_size")
    )]
    pub binary_content_path: Option<PathBuf>,

    #[clap(skip)]
    pub api: Option<Api>,
}

impl CreateCommand {
    pub async fn run(
        self,
        global_options: GlobalOptions,
    ) -> Result<Option<CreateBinaryPartResponse>, Error> {
        let expected_binary_size = if let Some(binary_content_path) = self.binary_content_path {
            let file = fs::File::open(binary_content_path).unwrap();
            file.metadata().unwrap().len()
        } else {
            self.expected_binary_size.unwrap()
        };

        let params = CreateBinaryPartParams {
            binary_prn: self.binary_prn,
            index: self.index,
            expected_binary_size,
            hash: self.hash,
            size: self.size,
        };

        let api = if let Some(api) = self.api {
            api
        } else {
            Api::new(ApiOptions {
                api_key: global_options.api_key.unwrap(),
                endpoint: global_options.base_url,
                ca_bundle_path: global_options.ca_path,
            })
        };

        api.binary_parts().create(params).await.context(ApiSnafu)
    }
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self.inner.run(global_options).await? {
            Some(binary_part) => print_json!(&binary_part),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[arg(long)]
    pub binary_prn: String,

    #[clap(skip)]
    pub api: Option<Api>,
}

impl ListCommand {
    pub async fn run(
        self,
        global_options: GlobalOptions,
    ) -> Result<Option<ListBinaryPartsResponse>, Error> {
        let params = ListBinaryPartsParams {
            binary_prn: self.binary_prn,
        };

        let api = if let Some(api) = self.api {
            api
        } else {
            Api::new(ApiOptions {
                api_key: global_options.api_key.unwrap(),
                endpoint: global_options.base_url,
                ca_bundle_path: global_options.ca_path,
            })
        };

        api.binary_parts().list(params).await.context(ApiSnafu)
    }
}

impl Command<ListCommand> {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self.inner.run(global_options).await? {
            Some(binary_part) => print_json!(&binary_part),
            None => panic!(),
        }

        Ok(())
    }
}
