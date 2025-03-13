mod profiles;

use std::fs;
use std::io::{BufWriter, Write};

use super::Command;
use crate::config::config_v1::ConfigV1;
use crate::config::config_v2::ConfigV2;
use crate::utils::StyledStr;
use crate::utils::{maybe_config_directory, Style};
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use profiles::ProfilesCommand;

#[derive(Parser, Debug)]
pub enum ConfigCommand {
    Upgrade(Command<UpgradeCommand>),
    #[command(subcommand)]
    Profiles(ProfilesCommand),
}

impl ConfigCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Upgrade(cmd) => cmd.run(global_options).await,
            Self::Profiles(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct UpgradeCommand;

impl Command<UpgradeCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let mut config_dir_path = if let Some(config_dir) = maybe_config_directory(&global_options)
        {
            config_dir
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
