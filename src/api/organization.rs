use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
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
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::AddUser(cmd) => cmd.run(global_options).await,
            Self::RemoveUser(cmd) => cmd.run(global_options).await,
            Self::GetUser(cmd) => cmd.run(global_options).await,
            Self::ListUsers(cmd) => cmd.run(global_options).await,
            Self::UpdateUser(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct AddUserCommand {
    /// The role the user has within the organization.
    #[arg(long)]
    role: String,

    /// The username of the user to add to the organization.
    #[arg(long)]
    username: String,
}

impl Command<AddUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = AddOrganizationUserParams {
            organization_name: global_options.organization_name.unwrap(),
            role: self.inner.role,
            username: self.inner.username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
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
    /// The username of the user to remove from the organization.
    #[arg(long)]
    user_username: String,
}

impl Command<RemoveUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = RemoveOrganizationUserParams {
            organization_name: global_options.organization_name.unwrap(),
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
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
    /// The username of the user to get from the organization.
    #[arg(long)]
    user_username: String,
}

impl Command<GetUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetOrganizationUserParams {
            organization_name: global_options.organization_name.unwrap(),
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
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
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListOrganizationUserParams {
            organization_name: global_options.organization_name.unwrap(),
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
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
    /// The role the user has within the organization.
    #[arg(long)]
    role: String,

    /// The username of the user to update within the organization.
    #[arg(long)]
    user_username: String,
}

impl Command<UpdateUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateOrganizationUserParams {
            organization_name: global_options.organization_name.unwrap(),
            role: self.inner.role,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
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
