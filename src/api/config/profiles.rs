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
}

impl ProfilesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
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
