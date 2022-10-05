use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use peridio_sdk::api::organization_users::{
    AddOrganizationUserParams, GetOrganizationUserParams, ListOrganizationUserParams,
    RemoveOrganizationUserParams, UpdateOrganizationUserParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
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

#[derive(StructOpt, Debug)]
pub struct AddUserCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    role: String,

    #[structopt(long)]
    username: String,
}

impl Command<AddUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = AddOrganizationUserParams {
            organization_name: self.inner.organization_name,
            role: self.inner.role,
            username: self.inner.username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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

#[derive(StructOpt, Debug)]
pub struct RemoveUserCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    user_username: String,
}

impl Command<RemoveUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = RemoveOrganizationUserParams {
            organization_name: self.inner.organization_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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

#[derive(StructOpt, Debug)]
pub struct GetUserCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    user_username: String,
}

impl Command<GetUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetOrganizationUserParams {
            organization_name: self.inner.organization_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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

#[derive(StructOpt, Debug)]
pub struct ListUsersCommand {
    #[structopt(long)]
    organization_name: String,
}

impl Command<ListUsersCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListOrganizationUserParams {
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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

#[derive(StructOpt, Debug)]
pub struct UpdateUserCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    role: String,

    #[structopt(long)]
    user_username: String,
}

impl Command<UpdateUserCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateOrganizationUserParams {
            organization_name: self.inner.organization_name,
            role: self.inner.role,
            user_username: self.inner.user_username,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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
