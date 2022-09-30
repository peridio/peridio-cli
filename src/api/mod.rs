mod deployments;
mod device_certificates;
mod devices;
mod firmwares;
mod organization_users;
mod product_users;
mod products;
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
    DeviceCertificates(device_certificates::DeviceCertificatesCommand),
    Firmwares(firmwares::FirmwaresCommand),
    OrganizationUsers(organization_users::OrganizationUsersCommand),
    Products(products::ProductsCommand),
    ProductUsers(product_users::ProductUsersCommand),
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
            ApiCommand::DeviceCertificates(cmd) => cmd.run().await?,
            ApiCommand::Firmwares(cmd) => cmd.run().await?,
            ApiCommand::OrganizationUsers(cmd) => cmd.run().await?,
            ApiCommand::Products(cmd) => cmd.run().await?,
            ApiCommand::ProductUsers(cmd) => cmd.run().await?,
            ApiCommand::SigningKeys(cmd) => cmd.run().await?,
            ApiCommand::Users(cmd) => cmd.run().await?,
            ApiCommand::Upgrade(cmd) => cmd.run().await?,
        };

        Ok(())
    }
}
