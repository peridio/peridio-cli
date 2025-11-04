mod artifact_versions;
mod artifacts;
mod binaries;
mod binary_parts;
mod binary_signatures;
mod bundle_overrides;
mod bundles;
mod ca_certificates;
mod cohorts;
mod config;
mod device_certificates;
mod devices;
mod products;
mod releases;
mod signing_keys;
mod tunnels;
mod upgrade;
mod users;
mod webhooks;
mod x509;
use crate::utils::Style;
use crate::utils::StyledStr;
use crate::GlobalOptions;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Command<T>
where
    T: Parser + clap::Args,
{
    #[command(flatten)]
    inner: T,
}

#[derive(clap::Subcommand, Debug)]
pub enum CliCommands {
    #[command(flatten)]
    ApiCommand(ApiCommand),
    /// Inspect the calling users's identity
    #[command(subcommand)]
    Users(users::UsersCommand),
    /// Upgrade the CLI
    #[command()]
    Upgrade(upgrade::UpgradeCommand),
    /// Manage the CLI's config
    #[command(subcommand)]
    Config(config::ConfigCommand),
    /// Create X.509 certificates and private keys
    #[command(subcommand)]
    X509(x509::X509Command),
}

#[derive(clap::Subcommand, Debug)]
pub enum ApiCommand {
    #[command(subcommand)]
    Artifacts(artifacts::ArtifactsCommand),
    #[command(subcommand)]
    ArtifactVersions(artifact_versions::ArtifactVersionsCommand),
    #[command(subcommand)]
    Bundles(bundles::BundlesCommand),
    #[command(subcommand)]
    BundleOverrides(bundle_overrides::BundleOverridesCommand),
    #[command(subcommand)]
    Binaries(binaries::BinariesCommand),
    #[command(subcommand)]
    BinaryParts(binary_parts::BinaryPartsCommand),
    #[command(subcommand)]
    BinarySignatures(binary_signatures::BinarySignaturesCommand),
    #[command(subcommand)]
    CaCertificates(ca_certificates::CaCertificatesCommand),
    #[command(subcommand)]
    Cohorts(cohorts::CohortsCommand),
    #[command(subcommand)]
    Devices(devices::DevicesCommand),
    #[command(subcommand)]
    DeviceCertificates(device_certificates::DeviceCertificatesCommand),
    #[command(subcommand)]
    Products(products::ProductsCommand),
    #[command(subcommand)]
    Releases(releases::ReleasesCommand),
    #[command(subcommand)]
    SigningKeys(signing_keys::SigningKeysCommand),
    #[command(subcommand)]
    Tunnels(tunnels::TunnelsCommand),
    #[command(subcommand)]
    Webhooks(webhooks::WebhooksCommand),
}

impl CliCommands {
    fn show_api_version_warning_if_needed(global_options: &GlobalOptions) {
        if global_options.api_version.is_none() {
            let mut warning = StyledStr::new();
            warning.push_str(Some(Style::Warning), "warning: ".to_string());
            warning.push_str(
                None,
                "No API version specified. Using default version 2.".to_string(),
            );
            warning.print_msg().unwrap();
        }
    }

    pub(crate) async fn run(self, global_options: GlobalOptions) -> Result<(), crate::Error> {
        match self {
            CliCommands::ApiCommand(api) => {
                // require api key
                let mut missing_arguments = Vec::new();

                if global_options.api_key.is_none() {
                    missing_arguments.push("--api-key".to_owned());
                }

                Self::print_missing_arguments(missing_arguments);

                Self::show_api_version_warning_if_needed(&global_options);

                match api {
                    ApiCommand::Artifacts(cmd) => cmd.run(global_options).await?,
                    ApiCommand::ArtifactVersions(cmd) => cmd.run(global_options).await?,
                    ApiCommand::Bundles(cmd) => cmd.run(global_options).await?,
                    ApiCommand::BundleOverrides(cmd) => cmd.run(global_options).await?,
                    ApiCommand::Binaries(cmd) => cmd.run(global_options).await?,
                    ApiCommand::BinaryParts(cmd) => cmd.run(global_options).await?,
                    ApiCommand::BinarySignatures(cmd) => cmd.run(global_options).await?,
                    ApiCommand::CaCertificates(cmd) => cmd.run(global_options).await?,
                    ApiCommand::Cohorts(cmd) => cmd.run(global_options).await?,
                    ApiCommand::DeviceCertificates(cmd) => cmd.run(global_options).await?,
                    ApiCommand::Devices(cmd) => cmd.run(global_options).await?,
                    ApiCommand::Products(cmd) => cmd.run(global_options).await?,
                    ApiCommand::Releases(cmd) => cmd.run(global_options).await?,
                    ApiCommand::SigningKeys(cmd) => cmd.run(global_options).await?,
                    ApiCommand::Tunnels(cmd) => cmd.run(global_options).await?,
                    ApiCommand::Webhooks(cmd) => cmd.run(global_options).await?,
                }
            }
            CliCommands::Users(cmd) => {
                Self::show_api_version_warning_if_needed(&global_options);
                cmd.run(global_options).await?
            }
            CliCommands::Upgrade(cmd) => cmd.run().await?,
            CliCommands::Config(cmd) => cmd.run(global_options).await?,
            CliCommands::X509(cmd) => cmd.run(global_options).await?,
        };

        Ok(())
    }

    pub(crate) fn print_missing_arguments(missing_arguments: Vec<String>) {
        if !missing_arguments.is_empty() {
            let mut error = StyledStr::new();

            error.push_str(Some(Style::Error), "error: ".to_string());
            error.push_str(
                None,
                "The following arguments are required:\r\n".to_string(),
            );
            for missing_argument in missing_arguments.iter() {
                error.push_str(Some(Style::Success), format!("\t{missing_argument}\r\n"));
            }
            error.push_str(None, "\r\nThey must be supplied either:\r\n".to_string());
            error.push_str(
                None,
                "\t- via the CLI config file and referenced by profile\r\n".to_string(),
            );
            error.push_str(
                None,
                "\t- directly to the top level command (not to subcommands)".to_string(),
            );
            error.print_data_err();
        }
    }
}
