use peridio_sdk::{
    api::{element, Error},
    Api,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct VersionCommand<T: StructOpt> {
    #[structopt(long)]
    pub element_id: String,

    #[structopt(flatten)]
    inner: T,
}

#[derive(StructOpt, Debug)]
pub enum ElementVersionCommand {
    Create(VersionCommand<CreateCommand>),
    Get(VersionCommand<GetCommand>),
    List(VersionCommand<ListCommand>),
}

impl ElementVersionCommand {
    pub async fn run(&self, api: Api) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(api).await,
            Self::Get(cmd) => cmd.run(api).await,
            Self::List(cmd) => cmd.run(api).await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    #[structopt(long)]
    number: String,
}

impl VersionCommand<CreateCommand> {
    async fn run(&self, api: Api) -> Result<(), Error> {
        let version = element::ElementVersionChangeset {
            number: self.inner.number.clone(),
        };

        let version = api
            .element(&self.element_id)
            .versions()
            .create(version)
            .await?;

        println!("{:?}", version);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    id: String,
}

impl VersionCommand<GetCommand> {
    async fn run(&self, api: Api) -> Result<(), Error> {
        let version = api
            .element(&self.element_id)
            .version(&self.inner.id)
            .get()
            .await?;

        println!("{:?}", version);

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {}

impl VersionCommand<ListCommand> {
    async fn run(&self, api: Api) -> Result<(), Error> {
        let versions = api.element(&self.element_id).versions().list().await?;

        println!("{:?}", versions);

        Ok(())
    }
}
