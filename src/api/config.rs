use std::collections::HashMap;
use std::fs;
use std::io::{stdin, stdout, BufWriter, Write};
use std::path::PathBuf;

use super::Command;
use crate::config::config_v1::ConfigV1;
use crate::config::config_v2::{ConfigV2, ProfileV2};
use crate::utils::Style;
use crate::utils::StyledStr;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use directories::ProjectDirs;

#[derive(Parser, Debug)]
pub enum ConfigCommand {
    Upgrade(Command<UpgradeCommand>),
    Init(Command<InitCommand>),
}

impl ConfigCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Upgrade(cmd) => cmd.run(global_options).await,
            Self::Init(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct UpgradeCommand;

impl Command<UpgradeCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let mut config_dir_path = if let Some(config_dir) = &global_options.config_directory {
            let config_dir_path = PathBuf::from(config_dir);

            if config_dir_path.exists() {
                // use this config
                config_dir_path
            } else {
                panic!("The provided config directory is invalid");
            }
        } else if let Some(proj_dirs) = ProjectDirs::from("", "", "peridio") {
            let cache_dir = proj_dirs.config_dir();

            cache_dir.to_path_buf()
        } else {
            panic!("We can't determine your config path")
        };

        config_dir_path.push("config.json");

        if config_dir_path.exists() {
            let config_file =
                fs::read_to_string(&config_dir_path).expect("Cannot read config file");

            if let Ok(config) = serde_json::from_str::<ConfigV1>(&config_file) {
                let config_v2: Result<ConfigV2, _> = config.try_into();
                if let Ok(configv2) = config_v2 {
                    let file = std::fs::OpenOptions::new()
                        .write(true)
                        .open(&config_dir_path)
                        .unwrap();
                    let mut writer = BufWriter::new(file);
                    serde_json::to_writer_pretty(&mut writer, &configv2).unwrap();
                    writer.flush().unwrap();

                    let mut msg = StyledStr::new();
                    msg.push_str(Some(Style::Success), "success: ".to_string());
                    msg.push_str(None, "The config file has been migrated to v2.".to_string());
                    msg.print_success();
                }
            } else if serde_json::from_str::<ConfigV2>(&config_file).is_ok() {
                eprintln!("Your config is up to date!");
            } else {
                panic!("Your current config file can't be upgraded automatically.");
            }
        } else {
            panic!(
                "We can't find any config.json file in you current directory {}",
                config_dir_path.display()
            )
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct InitCommand;

impl Command<InitCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        // Get configuration directory
        let config_dir_path = if let Some(config_dir) = &global_options.config_directory {
            let config_dir_path = PathBuf::from(config_dir);
            fs::create_dir_all(&config_dir_path).unwrap();
            config_dir_path
        } else if let Some(proj_dirs) = ProjectDirs::from("", "", "peridio") {
            let config_dir = proj_dirs.config_dir();
            fs::create_dir_all(config_dir).unwrap();
            config_dir.to_path_buf()
        } else {
            panic!("Unable to determine configuration directory")
        };

        // Create config and credentials paths
        let config_path = config_dir_path.join("config.json");
        let credentials_path = config_dir_path.join("credentials.json");

        // Prompt for profile name
        print!("Enter profile name (default: 'default'): ");
        stdout().flush().unwrap();
        let mut profile_name = String::new();
        stdin().read_line(&mut profile_name).unwrap();
        let profile_name = profile_name.trim();
        let profile_name = if profile_name.is_empty() {
            "default"
        } else {
            profile_name
        };

        // Prompt for organization name
        print!("Enter organization name: ");
        stdout().flush().unwrap();
        let mut organization_name = String::new();
        stdin().read_line(&mut organization_name).unwrap();
        let organization_name = organization_name.trim().to_string();

        // Prompt for API key
        print!("Enter API key: ");
        stdout().flush().unwrap();
        let mut api_key = String::new();
        stdin().read_line(&mut api_key).unwrap();
        let api_key = api_key.trim().to_string();

        // Create profile
        let profile = ProfileV2 {
            api_key: None,  // We'll store this in the credentials file
            base_url: None, // Use default
            ca_path: None,  // Use default
            organization_name: Some(organization_name),
        };

        // Load or create config file
        let mut config = if config_path.exists() {
            let config_data = fs::read_to_string(&config_path).expect("Cannot read config file");
            serde_json::from_str::<ConfigV2>(&config_data).unwrap_or_else(|_| ConfigV2::default())
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
        let mut credentials: HashMap<String, serde_json::Value> = if credentials_path.exists() {
            let creds_data =
                fs::read_to_string(&credentials_path).expect("Cannot read credentials file");
            serde_json::from_str(&creds_data).unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };

        // Add or update credentials
        let mut profile_creds = serde_json::Map::new();
        profile_creds.insert("api_key".to_string(), serde_json::Value::String(api_key));
        credentials.insert(
            profile_name.to_string(),
            serde_json::Value::Object(profile_creds),
        );

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

        // Success message - Print to stdout for tests and regular output
        println!(
            "Profile '{}' configured successfully.\nUse with: peridio -p {} [command]",
            profile_name, profile_name
        );

        // Also print styled message to stderr to maintain consistency with other commands
        let mut msg = StyledStr::new();
        msg.push_str(Some(Style::Success), "success: ".to_string());
        msg.push_str(
            None,
            format!(
                "Profile '{}' configured successfully.\nUse with: peridio -p {} [command]",
                profile_name, profile_name
            ),
        );
        let _ = msg.print_err(); // Ignore errors from printing to stderr

        Ok(())
    }
}
