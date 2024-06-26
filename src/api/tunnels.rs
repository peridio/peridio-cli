use super::Command;
use crate::api::list::ListArgs;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::tunnels::{CreateTunnelParams, ListTunnelsParams, UpdateTunnelParams};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum TunnelsCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
}

impl TunnelsCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// An optional list of CIDR blocks that can use the resource.
    #[arg(long)]
    cidr_block_allowlist: Option<Vec<String>>,

    /// The PRN of the device you wish to create the resource for.
    #[arg(long)]
    device_prn: String,

    /// The port of the device that being used for the service.
    #[arg(long)]
    device_tunnel_port: u16,

    /// The length of time in seconds for the tunnel to live.
    #[arg(long)]
    ttl: Option<u16>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateTunnelParams {
            cidr_block_allowlist: self.inner.cidr_block_allowlist,
            device_prn: self.inner.device_prn,
            device_tunnel_port: self.inner.device_tunnel_port,
            ttl: self.inner.ttl,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.tunnels().create(params).await.context(ApiSnafu)? {
            Some(tunnel) => print_json!(&tunnel),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[clap(flatten)]
    list_args: ListArgs,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListTunnelsParams {
            limit: self.inner.list_args.limit,
            order: self.inner.list_args.order,
            search: self.inner.list_args.search,
            page: self.inner.list_args.page,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.tunnels().list(params).await.context(ApiSnafu)? {
            Some(tunnel) => print_json!(&tunnel),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// The PRN of the resource to update.
    #[arg(long)]
    prn: String,

    /// The resource's state, currently only supports "closed".
    #[arg(long)]
    pub state: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateTunnelParams {
            prn: self.inner.prn,
            state: self.inner.state,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.tunnels().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
