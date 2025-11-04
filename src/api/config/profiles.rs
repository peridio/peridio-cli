use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::Command;
use crate::config::config_v2::{ConfigV2, ProfileV2};
use crate::config::Credential;
use crate::utils::StyledStr;
use crate::utils::{maybe_config_directory, Style};
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Password;

#[derive(Parser, Debug)]
pub enum ProfilesCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Show(Command<ShowCommand>),
    Update(Command<UpdateCommand>),
}

impl ProfilesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Show(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The profile's name.
    #[arg(long)]
    name: String,

    /// The profile's api_key. Can be omitted to supply the value interactively.
    #[arg(long, required_if_eq("no_input", "true"))]
    api_key: Option<String>,

    /// Disable interactivity.
    #[arg(long, default_value = "false")]
    no_input: bool,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        // Get configuration directory
        let config_dir_path = if let Some(config_dir) = maybe_config_directory(&global_options) {
            config_dir
        } else {
            panic!("We can't determine your config path")
        };
        fs::create_dir_all(&config_dir_path).unwrap();

        // Create config and credentials paths
        let config_path = config_dir_path.join("config.json");
        let credentials_path = config_dir_path.join("credentials.json");

        let profile_name = self.inner.name.trim().to_string();

        let api_key = if self.inner.no_input {
            self.inner.api_key.unwrap()
        } else {
            // Prompt for API key
            let api_key: String = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("API key")
                .interact()
                .unwrap()
                .trim()
                .to_string();

            api_key
        };

        // Create profile
        let profile = ProfileV2 {
            api_key: None,     // We'll store this in the credentials file
            base_url: None,    // Use default
            ca_path: None,     // Use default
            api_version: None, // Use default
        };

        // Load or create config file
        let mut config = if config_path.exists() {
            ConfigV2::try_from(&config_path).unwrap_or_else(|_| ConfigV2::default())
        } else {
            ConfigV2::default()
        };

        // Add or update profile
        config.profiles.insert(profile_name.to_string(), profile);

        // Write config file
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&config_path)
            .unwrap();
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &config).unwrap();
        writer.flush().unwrap();

        // Load or create credentials file
        let mut credentials: HashMap<String, Credential> = if credentials_path.exists() {
            let creds_data =
                fs::read_to_string(&credentials_path).expect("Cannot read credentials file");
            serde_json::from_str(&creds_data).unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };

        // Add or update credentials
        let profile_creds = Credential {
            api_key: Some(api_key),
        };
        credentials.insert(profile_name.to_string(), profile_creds);

        // Write credentials file
        let creds_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&credentials_path)
            .unwrap();
        let mut creds_writer = BufWriter::new(creds_file);
        serde_json::to_writer_pretty(&mut creds_writer, &credentials).unwrap();
        creds_writer.flush().unwrap();

        let mut msg = StyledStr::new();
        msg.push_str(Some(Style::Success), "success: ".to_string());

        let escaped_path = config_dir_path
            .into_os_string()
            .into_string()
            .unwrap()
            .replace(" ", "\\ ");

        let path = Path::new(&escaped_path);

        msg.push_str(
            None,
            format!(
                "Profile '{}' configured successfully at {}.\nUse with: peridio -p {} [command]",
                profile_name,
                path.display(),
                profile_name
            ),
        );
        msg.print_success();
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand;

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let config_dir_path = if let Some(config_dir) = maybe_config_directory(&global_options) {
            config_dir
        } else {
            panic!("We can't determine your config path")
        };

        let config_path = config_dir_path.join("config.json");

        if !config_path.exists() {
            let mut msg = StyledStr::new();
            msg.push_str(Some(Style::Warning), "warning: ".to_string());
            msg.push_str(
                None,
                "No config file found. Use 'peridio config profiles create' to create a profile."
                    .to_string(),
            );
            msg.print_msg().unwrap();
            return Ok(());
        }

        let config = ConfigV2::try_from(&config_path).unwrap_or_else(|_| ConfigV2::default());

        if config.profiles.is_empty() {
            let mut msg = StyledStr::new();
            msg.push_str(Some(Style::Warning), "warning: ".to_string());
            msg.push_str(
                None,
                "No profiles found. Use 'peridio config profiles create' to create a profile."
                    .to_string(),
            );
            msg.print_msg().unwrap();
            return Ok(());
        }

        println!("Available profiles:");
        for profile_name in config.profiles.keys() {
            println!("  {}", profile_name);
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ShowCommand {
    /// The profile name to show. If not provided, uses the current profile.
    profile_name: Option<String>,
}

impl Command<ShowCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let config_dir_path = if let Some(config_dir) = maybe_config_directory(&global_options) {
            config_dir
        } else {
            panic!("We can't determine your config path")
        };

        let config_path = config_dir_path.join("config.json");
        let credentials_path = config_dir_path.join("credentials.json");

        if !config_path.exists() {
            let mut msg = StyledStr::new();
            msg.push_str(Some(Style::Error), "error: ".to_string());
            msg.push_str(None, "No config file found.".to_string());
            msg.print_data_err();
        }

        let config = ConfigV2::try_from(&config_path).unwrap_or_else(|_| ConfigV2::default());

        let profile_name = if let Some(name) = &self.inner.profile_name {
            name.clone()
        } else if let Some(current_profile) = &global_options.profile {
            current_profile.clone()
        } else {
            let mut msg = StyledStr::new();
            msg.push_str(Some(Style::Error), "error: ".to_string());
            msg.push_str(None, "No profile specified and no current profile set. Use -p <profile> or provide a profile name.".to_string());
            msg.print_data_err();
        };

        let profile = match config.profiles.get(&profile_name) {
            Some(profile) => profile,
            None => {
                let mut msg = StyledStr::new();
                msg.push_str(Some(Style::Error), "error: ".to_string());
                msg.push_str(None, format!("Profile '{}' not found.", profile_name));
                msg.print_data_err();
            }
        };

        // Load credentials to show if API key is set
        let credentials: HashMap<String, Credential> = if credentials_path.exists() {
            let creds_data =
                fs::read_to_string(&credentials_path).expect("Cannot read credentials file");
            serde_json::from_str(&creds_data).unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };

        let has_api_key = credentials
            .get(&profile_name)
            .and_then(|c| c.api_key.as_ref())
            .is_some();

        println!("Profile: {}", profile_name);
        println!("  API Key: {}", if has_api_key { "set" } else { "not set" });
        println!(
            "  Base URL: {}",
            profile.base_url.as_ref().unwrap_or(&"default".to_string())
        );
        println!(
            "  CA Path: {}",
            profile.ca_path.as_ref().unwrap_or(&"default".to_string())
        );
        println!(
            "  API Version: {}",
            profile
                .api_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "default".to_string())
        );

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// The profile name to update.
    profile_name: String,

    /// Update the API key. Can be omitted to supply the value interactively.
    #[arg(long)]
    api_key: Option<String>,

    /// Update the base URL.
    #[arg(long)]
    base_url: Option<String>,

    /// Update the CA path.
    #[arg(long)]
    ca_path: Option<String>,

    /// Update the API version.
    #[arg(long)]
    api_version: Option<u8>,

    /// Disable interactivity.
    #[arg(long, default_value = "false")]
    no_input: bool,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let config_dir_path = if let Some(config_dir) = maybe_config_directory(&global_options) {
            config_dir
        } else {
            panic!("We can't determine your config path")
        };

        let config_path = config_dir_path.join("config.json");
        let credentials_path = config_dir_path.join("credentials.json");

        if !config_path.exists() {
            let mut msg = StyledStr::new();
            msg.push_str(Some(Style::Error), "error: ".to_string());
            msg.push_str(None, "No config file found. Create a profile first with 'peridio config profiles create'.".to_string());
            msg.print_data_err();
        }

        let mut config = ConfigV2::try_from(&config_path).unwrap_or_else(|_| ConfigV2::default());

        let profile_name = &self.inner.profile_name;
        let profile = match config.profiles.get_mut(profile_name) {
            Some(profile) => profile,
            None => {
                let mut msg = StyledStr::new();
                msg.push_str(Some(Style::Error), "error: ".to_string());
                msg.push_str(None, format!("Profile '{}' not found.", profile_name));
                msg.print_data_err();
            }
        };

        let mut updated = false;

        // Update base URL if provided
        if let Some(base_url) = &self.inner.base_url {
            profile.base_url = Some(base_url.clone());
            updated = true;
        }

        // Update CA path if provided
        if let Some(ca_path) = &self.inner.ca_path {
            profile.ca_path = Some(ca_path.clone());
            updated = true;
        }

        // Update API version if provided
        if let Some(api_version) = self.inner.api_version {
            profile.api_version = Some(api_version);
            updated = true;
        }

        // Handle API key update
        if self.inner.api_key.is_some()
            || (!self.inner.no_input
                && self.inner.api_key.is_none()
                && self.inner.base_url.is_none()
                && self.inner.ca_path.is_none()
                && self.inner.api_version.is_none())
        {
            let api_key = if let Some(api_key) = self.inner.api_key {
                api_key
            } else if !self.inner.no_input {
                // Prompt for API key
                let api_key: String = Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("New API key (leave empty to skip)")
                    .allow_empty_password(true)
                    .interact()
                    .unwrap()
                    .trim()
                    .to_string();

                if api_key.is_empty() {
                    String::new() // Will be handled below
                } else {
                    api_key
                }
            } else {
                String::new()
            };

            if !api_key.is_empty() {
                // Load or create credentials file
                let mut credentials: HashMap<String, Credential> = if credentials_path.exists() {
                    let creds_data = fs::read_to_string(&credentials_path)
                        .expect("Cannot read credentials file");
                    serde_json::from_str(&creds_data).unwrap_or_else(|_| HashMap::new())
                } else {
                    HashMap::new()
                };

                // Update credentials
                let profile_creds = Credential {
                    api_key: Some(api_key),
                };
                credentials.insert(profile_name.clone(), profile_creds);

                // Write credentials file
                let creds_file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&credentials_path)
                    .unwrap();
                let mut creds_writer = BufWriter::new(creds_file);
                serde_json::to_writer_pretty(&mut creds_writer, &credentials).unwrap();
                creds_writer.flush().unwrap();

                updated = true;
            }
        }

        if !updated {
            let mut msg = StyledStr::new();
            msg.push_str(Some(Style::Warning), "warning: ".to_string());
            msg.push_str(None, "No updates specified. Use --api-key, --base-url, --ca-path, or --api-version to update profile settings.".to_string());
            msg.print_msg().unwrap();
            return Ok(());
        }

        // Write updated config file
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&config_path)
            .unwrap();
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &config).unwrap();
        writer.flush().unwrap();

        let mut msg = StyledStr::new();
        msg.push_str(Some(Style::Success), "success: ".to_string());
        msg.push_str(
            None,
            format!("Profile '{}' updated successfully.", profile_name),
        );
        msg.print_success();
    }
}
