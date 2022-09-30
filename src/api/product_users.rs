use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::product_users::{
    AddProductUserParams, DeleteProductUserParams, GetProductUserParams, ListProductUserParams,
    UpdateProductUserParams,
};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum ProductUsersCommand {
    Add(Command<AddCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
}

impl ProductUsersCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Add(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Get(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
            Self::Update(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct AddCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(long)]
    role: String,

    #[structopt(long)]
    username: String,
}

impl Command<AddCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = AddProductUserParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            role: self.inner.role,
            username: self.inner.username,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.product_users().add(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
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

    #[structopt(long)]
    user_username: String,
}

impl Command<DeleteCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = DeleteProductUserParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(self.api_key, self.base_url);

        if (api.product_users().delete(params).await.context(ApiSnafu)?).is_some() {
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

    #[structopt(long)]
    user_username: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetProductUserParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.product_users().get(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = ListProductUserParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.product_users().list(params).await.context(ApiSnafu)? {
            Some(devices) => print_json!(&devices),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct UpdateCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(long)]
    role: String,

    #[structopt(long)]
    user_username: String,
}

impl Command<UpdateCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = UpdateProductUserParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            role: self.inner.role,
            user_username: self.inner.user_username,
        };

        let api = Api::new(self.api_key, self.base_url);

        match api.product_users().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
