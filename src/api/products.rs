use super::Command;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::products::CreateProductParams;
use peridio_sdk::api::products::GetProductParams;
use peridio_sdk::api::products::ListProductsParams;
use peridio_sdk::api::products::UpdateProductParams;
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum ProductsCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
}

impl ProductsCommand {
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
    /// Whether the product is archived.
    #[arg(long)]
    archived: Option<bool>,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    name: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateProductParams {
            archived: self.inner.archived,
            name: self.inner.name,
        };

        let api = Api::from(global_options);

        match api.products().create(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
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
        let params = ListProductsParams {
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

        match api.products().list(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
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
        value_parser = PRNValueParser::new(PRNType::Product)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetProductParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        match api.products().get(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
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
        value_parser = PRNValueParser::new(PRNType::Product)
    )]
    prn: String,
    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    name: Option<String>,
    /// Whether the product is archived.
    #[arg(long)]
    archived: Option<bool>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateProductParams {
            prn: self.inner.prn,
            name: self.inner.name,
            archived: self.inner.archived,
        };

        let api = Api::from(global_options);

        match api.products().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
