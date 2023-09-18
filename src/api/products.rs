use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::product_users::{
    AddProductUserParams, GetProductUserParams, ListProductUserParams, RemoveProductUserParams,
    UpdateProductUserParams,
};
use peridio_sdk::api::products::UpdateProduct;
use peridio_sdk::api::products::{
    CreateProductParams, DeleteProductParams, GetProductParams, ListProductParams,
    UpdateProductParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum ProductsCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
    AddUser(Command<AddUserCommand>),
    RemoveUser(Command<RemoveUserCommand>),
    GetUser(Command<GetUserCommand>),
    ListUsers(Command<ListUsersCommand>),
    UpdateUser(Command<UpdateUserCommand>),
}

impl ProductsCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
            Self::AddUser(cmd) => cmd.run(global_options).await,
            Self::RemoveUser(cmd) => cmd.run(global_options).await,
            Self::GetUser(cmd) => cmd.run(global_options).await,
            Self::ListUsers(cmd) => cmd.run(global_options).await,
            Self::UpdateUser(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    #[arg(long)]
    name: String,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateProductParams {
            organization_name: global_options.organization_name.unwrap(),
            name: self.inner.name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.products().create(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    #[arg(long)]
    product_name: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteProductParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        if (api.products().delete(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    product_name: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetProductParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.products().get(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListProductParams {
            organization_name: global_options.organization_name.unwrap(),
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.products().list(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    product_name: String,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateProductParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            product: UpdateProduct {
                name: self.inner.name,
            },
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.products().update(params).await.context(ApiSnafu)? {
            Some(product) => print_json!(&product),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct AddUserCommand {
    #[arg(long)]
    product_name: String,

    #[arg(long)]
    role: String,

    #[arg(long)]
    username: String,
}

impl Command<AddUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = AddProductUserParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            role: self.inner.role,
            username: self.inner.username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.product_users().add(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct RemoveUserCommand {
    #[arg(long)]
    product_name: String,

    #[arg(long)]
    user_username: String,
}

impl Command<RemoveUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = RemoveProductUserParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        if (api.product_users().remove(params).await.context(ApiSnafu)?).is_some() {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetUserCommand {
    #[arg(long)]
    product_name: String,

    #[arg(long)]
    user_username: String,
}

impl Command<GetUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetProductUserParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.product_users().get(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListUsersCommand {
    #[arg(long)]
    product_name: String,
}

impl Command<ListUsersCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListProductUserParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.product_users().list(params).await.context(ApiSnafu)? {
            Some(devices) => print_json!(&devices),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateUserCommand {
    #[arg(long)]
    product_name: String,

    #[arg(long)]
    role: String,

    #[arg(long)]
    user_username: String,
}

impl Command<UpdateUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateProductUserParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            role: self.inner.role,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.product_users().update(params).await.context(ApiSnafu)? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
