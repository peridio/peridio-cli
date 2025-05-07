use std::thread::sleep;
use std::time::{Duration, Instant};

use super::Command;
use crate::api::list::ListArgs;
use crate::print_json;
use crate::utils::{PRNType, PRNValueParser};
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use clap_num::number_range;
use peridio_sdk::api::tunnels::{
    CreateTunnelParams, GetTunnelParams, ListTunnelsParams, UpdateTunnelParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum TunnelsCommand {
    Create(Command<CreateCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
}

impl TunnelsCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// An optional list of CIDR blocks that can use the resource.
    ///
    /// Values can be provided by passing each value in a flag
    /// or by delimiting all values with ","
    #[arg(long, value_delimiter = ',')]
    cidr_block_allowlist: Option<Vec<String>>,

    /// The PRN of the device you wish to create the resource for.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Device)
    )]
    device_prn: String,

    /// The port of the device that being used for the service.
    #[arg(long)]
    device_tunnel_port: u16,

    /// The length of time in seconds for the tunnel to live.
    #[arg(long)]
    ttl: Option<u16>,

    /// Number of seconds to wait for a state other than "requested" (1..3600)
    #[arg(long, value_parser=less_than_3600)]
    wait: Option<u16>,
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
            Some(response) => {
                let max_time: u64 = self.inner.wait.unwrap_or(0).into();
                let now = Instant::now();

                if max_time > 0 {
                    let mut attempts: u64 = 0;

                    loop {
                        let sleep_secs = (attempts + 2).pow(2);
                        attempts += 1;

                        let params = GetTunnelParams {
                            prn: response.tunnel.prn.clone(),
                        };

                        match api.tunnels().get(params).await.context(ApiSnafu)? {
                            Some(response) => {
                                if response.tunnel.state != "requested" {
                                    print_json!(&response);
                                    break;
                                }

                                let next_sleep_secs = (attempts + 2).pow(2);

                                if now.elapsed().as_secs() + next_sleep_secs > max_time {
                                    print_json!(&response);
                                    break;
                                } else {
                                    sleep(Duration::from_secs(sleep_secs))
                                }
                            }
                            None => panic!(),
                        }
                    }
                } else {
                    print_json!(&response);
                }
            }
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    /// The PRN of the resource to get.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Tunnel)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetTunnelParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.tunnels().get(params).await.context(ApiSnafu)? {
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
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Tunnel)
    )]
    prn: String,

    /// The resource's state, currently only supports "closed".
    #[arg(long, value_parser=only_closed)]
    pub state: Option<String>,

    /// Number of seconds to extend the tunnel for (1..3600)
    #[arg(long, value_parser=less_than_3600)]
    pub ttl: Option<u16>,
}

fn less_than_3600(s: &str) -> Result<u16, String> {
    number_range(s, 1, 3600)
}

fn only_closed(s: &str) -> Result<String, String> {
    if s == "closed" {
        Ok(s.to_string())
    } else {
        Err("only accepts \"closed\"".to_string())
    }
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateTunnelParams {
            prn: self.inner.prn,
            state: self.inner.state,
            ttl: self.inner.ttl,
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
