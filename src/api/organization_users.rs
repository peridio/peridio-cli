use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::organization_users::{
    AddOrganizationUserParams, GetOrganizationUserParams, ListOrganizationUserParams,
    RemoveOrganizationUserParams, UpdateOrganizationUserParams,
};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum OrganizationUsersCommand {
    Add(Command<AddCommand>),
    Remove(Command<RemoveCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
}

impl OrganizationUsersCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Add(cmd) => cmd.run().await,
            Self::Remove(cmd) => cmd.run().await,
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
    role: String,

    #[structopt(long)]
    username: String,
}

impl Command<AddCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = AddOrganizationUserParams {
            organization_name: self.inner.organization_name,
            role: self.inner.role,
            username: self.inner.username,
        };

        let api = Api::new(self.api_key, self.base_url);

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
pub struct RemoveCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    user_username: String,
}

impl Command<RemoveCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = RemoveOrganizationUserParams {
            organization_name: self.inner.organization_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(self.api_key, self.base_url);

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
pub struct GetCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    user_username: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetOrganizationUserParams {
            organization_name: self.inner.organization_name,
            user_username: self.inner.user_username,
        };

        let api = Api::new(self.api_key, self.base_url);

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
pub struct ListCommand {
    #[structopt(long)]
    organization_name: String,
}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = ListOrganizationUserParams {
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(self.api_key, self.base_url);

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
pub struct UpdateCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    role: String,

    #[structopt(long)]
    user_username: String,
}

impl Command<UpdateCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = UpdateOrganizationUserParams {
            organization_name: self.inner.organization_name,
            role: self.inner.role,
            user_username: self.inner.user_username,
        };

        let api = Api::new(self.api_key, self.base_url);

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
