use clap::Parser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use crate::print_json;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use peridio_sdk::api::binary_parts::CreateBinaryPartParams;
use peridio_sdk::api::binary_parts::ListBinaryPartsParams;
use snafu::ResultExt;
use super::Command;

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
    binary_prn: String,
    #[arg(long)]
    expected_binary_size: u64,
    #[arg(long)]
    hash: String,
    #[arg(long)]
    index: u16,
    #[arg(long)]
    size: u64,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateBinaryPartParams {
            binary_prn: self.inner.binary_prn,
            index: self.inner.index,
            expected_binary_size: self.inner.expected_binary_size,
            hash: self.inner.hash,
            size: self.inner.size,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.binary_parts().create(params).await.context(ApiSnafu)? {
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
    #[arg(long)]
    pub limit: Option<u8>,
    #[arg(long)]
    pub order: Option<String>,
    #[arg(long)]
    pub page: Option<String>,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListBinaryPartsParams {
            binary_prn: self.inner.binary_prn,
            limit: self.inner.limit,
            order: self.inner.order,
            page: self.inner.page,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.binary_parts().list(params).await.context(ApiSnafu)? {
            Some(binary_part) => print_json!(&binary_part),
            None => panic!(),
        }

        Ok(())
    }
}
