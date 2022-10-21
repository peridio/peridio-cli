use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use clap::Parser;
use peridio_sdk::api::organization_users::{
    AddOrganizationUserParams, GetOrganizationUserParams, ListOrganizationUserParams,
    RemoveOrganizationUserParams, UpdateOrganizationUserParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum OrganizationCommand {
    AddUser(Command<AddUserCommand>),
    RemoveUser(Command<RemoveUserCommand>),
    GetUser(Command<GetUserCommand>),
    ListUsers(Command<ListUsersCommand>),
    UpdateUser(Command<UpdateUserCommand>),
}

impl OrganizationCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::AddUser(cmd) => cmd.run().await,
            Self::RemoveUser(cmd) => cmd.run().await,
            Self::GetUser(cmd) => cmd.run().await,
            Self::ListUsers(cmd) => cmd.run().await,
            Self::UpdateUser(cmd) => cmd.run().await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct AddUserCommand {
    #[arg(long)]
    role: String,

    #[arg(long)]
    username: String,
}

impl Command<AddUserCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = AddOrganizationUserParams {
            organization_name: self.organization_name,
            role: self.inner.role,
            username: self.inner.username,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api
            .organization_users()
            .add(params)
            .await
            .context(ApiSnafu)?
        {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct RemoveUserCommand {
    #[arg(long)]
    user_username: String,
}

impl Command<RemoveUserCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = RemoveOrganizationUserParams {
            organization_name: self.organization_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        if (api
            .organization_users()
            .remove(params)
            .await
            .context(ApiSnafu)?)
        .is_some()
        {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetUserCommand {
    #[arg(long)]
    user_username: String,
}

impl Command<GetUserCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetOrganizationUserParams {
            organization_name: self.organization_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api
            .organization_users()
            .get(params)
            .await
            .context(ApiSnafu)?
        {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListUsersCommand {}

impl Command<ListUsersCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = ListOrganizationUserParams {
            organization_name: self.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api
            .organization_users()
            .list(params)
            .await
            .context(ApiSnafu)?
        {
            Some(devices) => print_json!(&devices),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateUserCommand {
    #[arg(long)]
    role: String,

    #[arg(long)]
    user_username: String,
}

impl Command<UpdateUserCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = UpdateOrganizationUserParams {
            organization_name: self.organization_name,
            role: self.inner.role,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: self.api_key,
            endpoint: self.base_url,
            ca_bundle_path: self.ca_path,
        });

        match api
            .organization_users()
            .update(params)
            .await
            .context(ApiSnafu)?
        {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
