use super::element_version::ElementVersionCommand;
use super::Command;
use peridio_sdk::{
    api::{element, Error},
    Api,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum ElementCommand {
    /// Create an element
    Create(CreateCommand),

    /// Update an element
    Update(UpdateCommand),

    /// List elements
    List(ListCommand),

    /// Get an element
    Get(GetCommand),

    /// Operate on versions
    Versions(ElementVersionCommand),
}

impl Command<ElementCommand> {
    pub async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);

        match &self.inner {
            ElementCommand::Create(cmd) => cmd.run(api).await,
            ElementCommand::Update(cmd) => cmd.run(api).await,
            ElementCommand::List(cmd) => cmd.run(api).await,
            ElementCommand::Get(cmd) => cmd.run(api).await,
            ElementCommand::Versions(cmd) => cmd.run(api).await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    /// An element name
    #[structopt(long)]
    name: String,
}

impl CreateCommand {
    async fn run(&self, api: Api) -> Result<(), Error> {
        let element = element::ElementChangeset {
            name: self.name.clone(),
        };

        let element = api.elements().create(element).await?;
        println!("{:?}", element);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct UpdateCommand {
    /// An element id
    #[structopt(long)]
    id: String,

    /// An element name
    #[structopt(long)]
    name: String,
}

impl UpdateCommand {
    async fn run(&self, api: Api) -> Result<(), Error> {
        let changeset = element::ElementChangeset {
            name: self.name.clone(),
        };

        let element = api.element(&self.id).update(changeset).await?;
        println!("{:?}", element);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {}

impl ListCommand {
    async fn run(&self, api: Api) -> Result<(), Error> {
        let elements = api.elements().list().await?;
        println!("{:?}", elements);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    id: String,
}

impl GetCommand {
    async fn run(&self, api: Api) -> Result<(), Error> {
        let element = api.element(&self.id).get().await?;
        println!("{:?}", element);

        Ok(())
    }
}
