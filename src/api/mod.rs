mod deployments;
mod devices;
mod firmwares;
mod signing_keys;
mod upgrade;
mod users;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Command<T: StructOpt> {
    #[structopt(long, env = "PERIDIO_API_KEY")]
    pub api_key: String,

    #[structopt(long, env = "PERIDIO_BASE_URL")]
    pub base_url: Option<String>,

    #[structopt(flatten)]
    inner: T,
}

#[derive(StructOpt, Debug)]
pub enum ApiCommand {
    Deployments(deployments::DeploymentsCommand),
    Devices(devices::DevicesCommand),
    Firmwares(firmwares::FirmwaresCommand),
    SigningKeys(signing_keys::SigningKeysCommand),
    #[structopt(flatten)]
    Upgrade(upgrade::UpgradeCommand),
    Users(users::UsersCommand),
}

impl ApiCommand {
    pub(crate) async fn run(self) -> Result<(), crate::Error> {
        match self {
            ApiCommand::Deployments(cmd) => cmd.run().await?,
            ApiCommand::Devices(cmd) => cmd.run().await?,
            ApiCommand::Firmwares(cmd) => cmd.run().await?,
            ApiCommand::SigningKeys(cmd) => cmd.run().await?,
            ApiCommand::Users(cmd) => cmd.run().await?,
            ApiCommand::Upgrade(cmd) => cmd.run().await?,
        };

        Ok(())
    }
}
