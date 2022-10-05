use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use peridio_sdk::api::products::{
    CreateProductParams, DeleteProductParams, GetProductParams, ListProductParams,
    UpdateProductParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use peridio_sdk::api::UpdateProduct;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum ProductsCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
}

impl ProductsCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    #[structopt(long)]
    delta_updatable: Option<bool>,

    #[structopt(long)]
    name: String,

    #[structopt(long)]
    organization_name: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateProductParams {
            organization_name: self.inner.organization_name,
            name: self.inner.name,
            delta_updatable: self.inner.delta_updatable,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
        });

        match api.products().create(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct DeleteCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteProductParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
        });

        if (api.products().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetProductParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
        });

        match api.products().get(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {
    #[structopt(long)]
    organization_name: String,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListProductParams {
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
        });

        match api.products().list(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct UpdateCommand {
    #[structopt(long)]
    delta_updatable: Option<bool>,

    #[structopt(long)]
    name: Option<String>,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateProductParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            product: UpdateProduct {
                delta_updatable: self.inner.delta_updatable,
                name: self.inner.name,
            },
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
        });

        match api.products().update(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
            None => panic!(),
        }

        Ok(())
    }
}
