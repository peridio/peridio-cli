use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use crate::NonExistingPathSnafu;
use clap::Parser;
use peridio_sdk::api::binaries::BinaryState;
use peridio_sdk::api::binaries::CreateBinaryParams;
use peridio_sdk::api::binaries::GetBinaryParams;
use peridio_sdk::api::binaries::ListBinariesParams;
use peridio_sdk::api::binaries::UpdateBinaryParams;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use std::path::PathBuf;
use std::str::FromStr;
use std::{fs, io};

#[derive(Parser, Debug)]
pub enum BinariesCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
}

impl BinariesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]

pub struct CreateCommand {
    #[arg(long)]
    artifact_version_prn: String,
    #[arg(long)]
    description: Option<String>,
    #[arg(
        long,
        conflicts_with("content_path"),
        required_unless_present("content_path")
    )]
    hash: Option<String>,
    #[arg(
        long,
        conflicts_with("content_path"),
        required_unless_present("content_path")
    )]
    size: Option<u64>,
    #[arg(long)]
    target: String,
    #[arg(
        long,
        conflicts_with_all(["hash", "size"]),
        required_unless_present_any(["hash", "size"])
    )]
    content_path: Option<PathBuf>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let (size, hash) = if let Some(content_path) = self.inner.content_path {
            let mut file = fs::File::open(&content_path).context(NonExistingPathSnafu {
                path: &content_path,
            })?;
            let mut hasher = Sha256::new();
            let _ = io::copy(&mut file, &mut hasher).unwrap();
            let hash = hasher.finalize();
            (file.metadata().unwrap().len(), format!("{hash:X}"))
        } else {
            (self.inner.size.unwrap(), self.inner.hash.unwrap())
        };

        let params = CreateBinaryParams {
            artifact_version_prn: self.inner.artifact_version_prn,
            description: self.inner.description,
            hash,
            size,
            target: self.inner.target,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.binaries().create(params).await.context(ApiSnafu)? {
            Some(binary) => print_json!(&binary),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[arg(long)]
    pub limit: Option<u8>,
    #[arg(long)]
    pub order: Option<String>,
    #[arg(long)]
    pub search: String,
    #[arg(long)]
    pub page: Option<String>,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListBinariesParams {
            limit: self.inner.limit,
            order: self.inner.order,
            search: self.inner.search,
            page: self.inner.page,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.binaries().list(params).await.context(ApiSnafu)? {
            Some(binary) => print_json!(&binary),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetBinaryParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.binaries().get(params).await.context(ApiSnafu)? {
            Some(binary) => print_json!(&binary),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    #[arg(long)]
    prn: String,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long, value_parser = BinaryState::from_str)]
    pub state: Option<BinaryState>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateBinaryParams {
            prn: self.inner.prn,
            description: self.inner.description,
            state: self.inner.state,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.binaries().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
