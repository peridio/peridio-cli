use super::Command;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::utils::sdk_extensions::{ApiExt, ListExt};
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
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
use peridio_sdk::list_params::ListParams;
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
    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,
    /// The events that will trigger the webhook.
    ///
    /// Supply the flag multiple times to add multiple events.
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    enabled_events: Option<Vec<String>>,
    /// The URL that the webhook will send a POST request to.
    #[arg(long)]
    url: String,
    /// The PRN of the organization you wish to create the resource within.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Organization)
    )]
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

        let api = Api::from_options(global_options);

        match api.webhooks().create(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
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
        let params = ListWebhooksParams {
            list: ListParams::from_args(&self.inner.list_args),
        };

        let api = Api::from_options(global_options);

        match api.webhooks().list(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
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
        value_parser = PRNValueParser::new(PRNType::Webhook)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetWebhookParams {
            prn: self.inner.prn,
        };

        let api = Api::from_options(global_options);

        match api.webhooks().get(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct RollSecretCommand {
    /// The PRN of the resource to roll the secret for.
    #[arg(long)]
    prn: String,
}

impl Command<RollSecretCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = RollSecretWebhookParams {
            prn: self.inner.prn,
        };

        let api = Api::from_options(global_options);

        match api.webhooks().roll_secret(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct TestFireCommand {
    /// The PRN of the resource to test fire.
    #[arg(long)]
    prn: String,
}

impl Command<TestFireCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = TestFireWebhookParams {
            prn: self.inner.prn,
        };

        let api = Api::from_options(global_options);

        match api.webhooks().test_fire(params).await.context(ApiSnafu)? {
            Some(webhook) => print_json!(&webhook),
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
        value_parser = PRNValueParser::new(PRNType::Webhook)
    )]
    prn: String,
    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    pub description: Option<String>,
    /// The URL that the webhook will send a POST request to.
    #[arg(long)]
    pub url: Option<String>,
    /// The state of the webhook.
    #[arg(long)]
    pub state: Option<String>,
    /// The events that will trigger the webhook.
    ///
    /// Supply the flag multiple times to add multiple events.
    #[arg(long, num_args = 0.., value_delimiter = ',')]
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

        let api = Api::from_options(global_options);

        match api.webhooks().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The PRN of the resource to delete.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Webhook)
    )]
    webhook_prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteWebhookParams {
            webhook_prn: self.inner.webhook_prn,
        };

        let api = Api::from_options(global_options);

        if (api.webhooks().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}
