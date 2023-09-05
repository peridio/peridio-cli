mod config_v1;
mod config_v2;

use crate::config::config_v2::ConfigV2;
use crate::config::config_v2::ProfileV2;
use crate::utils::{Style, StyledStr};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Serialize, Deserialize, Clone)]
struct Credential {
    api_key: Option<String>,
}

pub struct Config;

impl Config {
    /// Attempt to fetch a profile by `profile_name` from `config`.
    pub fn get_profile(config: ConfigV2, profile_name) -> Result<ProfileV2, Err> {
            match  config.profiles.get(profile_name) {
              Some(profile) => Some(profile),
              None => {
                let mut error = StyledStr::new();
                error.push_str(Some(Style::Error), "error: ".to_string());
                error.push_str(None, "Profile ".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(Some(Style::Warning), profile_name.to_string());
                error.push_str(None, "'".to_string());
                error.push_str(None, " not found in ".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(
                    Some(Style::Warning),
                    format!(
                        "{}",
                        fs::canonicalize(&config_dir_path)
                            .unwrap()
                            .to_string_lossy()
                    ),
                );
                error.push_str(None, "'".to_string());
                error.print_data_err()
              }
            }
        // for (key, config) in config.profiles.iter_mut() {
        //     if let Some(credential) = credentials.get(key) {
        //         if let Some(api_key) = &credential.api_key {
        //             config.api_key = Some(api_key.to_string())
        //         } else {
        //             config.api_key = None
        //         }
        //     }
        // }
        // } else if profile.is_some() {
        //     let mut error = StyledStr::new();
        //
        //     error.push_str(Some(Style::Error), "error: ".to_string());
        //     error.push_str(None, "Config file not found at ".to_string());
        //
        //     // pop the config, so we can canonicalize the path since it already exist
        //     config_dir_path.pop();
        //     error.push_str(None, "'".to_string());
        //     error.push_str(
        //         Some(Style::Warning),
        //         format!(
        //             "{}/{}",
        //             fs::canonicalize(&config_dir_path)
        //                 .unwrap()
        //                 .to_string_lossy(),
        //             "config.json"
        //         ),
        //     );
        //     error.push_str(None, "'".to_string());
        //     error.print_data_err()
        // }
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

        let mut config: ConfigV2 = if config_dir_path.exists() {
            let config_file =
                fs::read_to_string(&config_dir_path).expect("Cannot read config file");
            serde_json::from_str(&config_file).expect("Cannot read config file")
        } else {
            ConfigV2::default()
        };
        config
    }
}
