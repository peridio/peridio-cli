use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::webhooks::CreateWebhookParams;
use peridio_sdk::api::webhooks::DeleteWebhookParams;
use peridio_sdk::api::webhooks::GetWebhookParams;
use peridio_sdk::api::webhooks::ListWebhooksParams;
use peridio_sdk::api::webhooks::RollSecretWebhookParams;
use peridio_sdk::api::webhooks::TestFireWebhookParams;
use peridio_sdk::api::webhooks::UpdateWebhookParams;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum WebhooksCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    RollSecret(Command<RollSecretCommand>),
    TestFire(Command<TestFireCommand>),
    Update(Command<UpdateCommand>),
}

impl WebhooksCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::RollSecret(cmd) => cmd.run(global_options).await,
            Self::TestFire(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]

pub struct CreateCommand {
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    enabled_events: Option<Vec<String>>,
    #[arg(long)]
    url: String,
    #[arg(long)]
    organization_prn: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateWebhookParams {
            description: self.inner.description,
            organization_prn: self.inner.organization_prn,
            enabled_events: self.inner.enabled_events,
            url: self.inner.url,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.webhooks().create(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
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
        let params = ListWebhooksParams {
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

        match api.webhooks().list(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
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
        let params = GetWebhookParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.webhooks().get(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct RollSecretCommand {
    #[arg(long)]
    prn: String,
}

impl Command<RollSecretCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = RollSecretWebhookParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.webhooks().roll_secret(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct TestFireCommand {
    #[arg(long)]
    prn: String,
}

impl Command<TestFireCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = TestFireWebhookParams {
            prn: self.inner.prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.webhooks().test_fire(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
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
    #[arg(long)]
    pub url: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub enabled_events: Option<Vec<String>>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateWebhookParams {
            prn: self.inner.prn,
            description: self.inner.description,
            enabled_events: self.inner.enabled_events,
            state: self.inner.state,
            url: self.inner.url,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.webhooks().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    #[arg(long)]
    webhook_prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteWebhookParams {
            webhook_prn: self.inner.webhook_prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        if (api.webhooks().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}
