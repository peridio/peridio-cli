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
}

impl Command<ElementCommand> {
    pub async fn run(self) -> Result<(), Error> {
        let api = Api::new(self.api_key, self.base_url);

        match &self.inner {
            ElementCommand::Create(cmd) => cmd.run(api).await,
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
