mod ca_certificates;
mod deployments;
mod device_certificates;
mod devices;
mod firmwares;
mod organization;
mod products;
mod signing_keys;
mod upgrade;
mod users;

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Command<T>
where
    T: Parser + clap::Args,
{
    #[arg(from_global)]
    api_key: String,

    #[arg(from_global)]
    base_url: Option<String>,

    #[arg(from_global)]
    ca_path: Option<PathBuf>,

    #[arg(from_global)]
    organization_name: String,

    #[command(flatten)]
    inner: T,
}

#[derive(clap::Subcommand, Debug)]
pub enum ApiCommand {
    #[command(subcommand)]
    CaCertificates(ca_certificates::CaCertificatesCommand),
    #[command(subcommand)]
    Deployments(deployments::DeploymentsCommand),
    #[command(subcommand)]
    Devices(devices::DevicesCommand),
    #[command(subcommand)]
    DeviceCertificates(device_certificates::DeviceCertificatesCommand),
    #[command(subcommand)]
    Firmwares(firmwares::FirmwaresCommand),
    #[command(subcommand)]
    Organizations(organization::OrganizationCommand),
    #[command(subcommand)]
    Products(products::ProductsCommand),
    #[command(subcommand)]
    SigningKeys(signing_keys::SigningKeysCommand),
    #[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
    Upgrade(upgrade::UpgradeCommand),
    #[command(subcommand)]
    Users(users::UsersCommand),
}

impl ApiCommand {
    pub(crate) async fn run(self) -> Result<(), crate::Error> {
        match self {
            ApiCommand::CaCertificates(cmd) => cmd.run().await?,
            ApiCommand::Deployments(cmd) => cmd.run().await?,
            ApiCommand::Devices(cmd) => cmd.run().await?,
            ApiCommand::DeviceCertificates(cmd) => cmd.run().await?,
            ApiCommand::Firmwares(cmd) => cmd.run().await?,
            ApiCommand::Organizations(cmd) => cmd.run().await?,
            ApiCommand::Products(cmd) => cmd.run().await?,
            ApiCommand::SigningKeys(cmd) => cmd.run().await?,
            ApiCommand::Users(cmd) => cmd.run().await?,
            ApiCommand::Upgrade(cmd) => cmd.run().await?,
        };

        Ok(())
    }
}
