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

use crate::GlobalOptions;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Command<T: StructOpt> {
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
    pub(crate) async fn run(self, global_options: GlobalOptions) -> Result<(), crate::Error> {
        match self {
            ApiCommand::Deployments(cmd) => cmd.run(global_options).await?,
            ApiCommand::Devices(cmd) => cmd.run(global_options).await?,
            ApiCommand::DeviceCertificates(cmd) => cmd.run(global_options).await?,
            ApiCommand::Firmwares(cmd) => cmd.run(global_options).await?,
            ApiCommand::OrganizationUsers(cmd) => cmd.run(global_options).await?,
            ApiCommand::Products(cmd) => cmd.run(global_options).await?,
            ApiCommand::ProductUsers(cmd) => cmd.run(global_options).await?,
            ApiCommand::SigningKeys(cmd) => cmd.run(global_options).await?,
            ApiCommand::Users(cmd) => cmd.run(global_options).await?,
            ApiCommand::Upgrade(cmd) => cmd.run().await?,
        };

        Ok(())
    }
}
