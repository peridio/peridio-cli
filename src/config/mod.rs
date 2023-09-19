pub(crate) mod config_v1;
pub(crate) mod config_v2;

use crate::config::config_v2::ConfigV2;
use crate::config::config_v2::ProfileV2;
use crate::utils::{Style, StyledStr};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

use self::config_v1::ConfigV1;

#[derive(Serialize, Deserialize, Clone)]
struct Credential {
    api_key: Option<String>,
}

pub struct Config;

impl Config {
    /// Attempt to fetch a profile by `profile_name` from `config`.
    pub fn get_profile(
        config: &ConfigV2,
        profile_name: &String,
    ) -> Result<ProfileV2, crate::Error> {
        match config.profiles.get(profile_name) {
            Some(profile) => Ok(profile.to_owned()),
            None => {
                let mut error = StyledStr::new();
                error.push_str(Some(Style::Error), "error: ".to_string());
                error.push_str(None, "Profile ".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(Some(Style::Warning), profile_name.to_string());
                error.push_str(None, "'".to_string());
                error.push_str(None, " not found.".to_string());
                error.print_data_err()
            }
        }
    }

    pub fn parse(config_directory: &Option<String>) -> Option<ConfigV2> {
        // get directory
        let mut config_dir_path = if let Some(config_dir) = config_directory {
            let config_dir_path = PathBuf::from(config_dir);

            if config_dir_path.exists() {
                // use this config
                config_dir_path
            } else {
                panic!("The provided config directory is invalid");
            }
        } else if let Some(proj_dirs) = ProjectDirs::from("", "", "peridio") {
            let cache_dir = proj_dirs.config_dir();

            fs::create_dir_all(cache_dir).unwrap();

            cache_dir.to_path_buf()
        } else {
            panic!("We can't determine your config path")
        };

        // get credentials
        config_dir_path.push("credentials.json");

        let credentials: HashMap<String, Credential> = if config_dir_path.exists() {
            let credentials_file =
                fs::read_to_string(&config_dir_path).expect("Cannot read credentials file");
            serde_json::from_str(&credentials_file).expect("Cannot read credential file")
        } else {
            HashMap::new()
        };

        // get config
        config_dir_path.pop();
        config_dir_path.push("config.json");

        if config_dir_path.exists() {
            let config_file =
                fs::read_to_string(&config_dir_path).expect("Cannot read config file");

            if serde_json::from_str::<ConfigV1>(&config_file).is_ok() {
                let mut error = StyledStr::new();
                error.push_str(Some(Style::Error), "error: ".to_string());
                error.push_str(None, "Your current config file is deprecated. Please upgrade your config by running:\r\n".to_string());
                error.push_str(Some(Style::Success), "\tperidio config upgrade".to_string());
                error.print_data_err();
            }

            let mut config: ConfigV2 =
                serde_json::from_str(&config_file).expect("Cannot read config file");

            for (profile_name, profile) in config.profiles.iter_mut() {
                if let Some(credential) = credentials.get(profile_name) {
                    profile.api_key = credential.api_key.clone();
                }
            }

            Some(config)
        } else {
            None
        }
    }
}
