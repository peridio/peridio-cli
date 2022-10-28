use std::{collections::HashMap, fs, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::utils::{Style, StyledStr};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub api_key: Option<String>,

    pub base_url: Option<String>,

    pub ca_path: Option<String>,

    pub organization_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Credential {
    api_key: Option<String>,
}

impl Config {
    pub fn read_config_file(
        profile: &Option<String>,
        config_directory: &Option<String>,
    ) -> Option<Self> {
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

        let mut config: HashMap<String, Self> = if config_dir_path.exists() {
            let config_file =
                fs::read_to_string(&config_dir_path).expect("Cannot read config file");
            serde_json::from_str(&config_file).expect("Cannot read config file")
        } else if profile.is_some() {
            let mut error = StyledStr::new();

            error.push_str(Some(Style::Error), "error: ".to_string());
            error.push_str(None, "Config file not found at ".to_string());

            // pop the config, so we can canonicalize the path since it already exist
            config_dir_path.pop();
            error.push_str(None, "'".to_string());
            error.push_str(
                Some(Style::Warning),
                format!(
                    "{}/{}",
                    fs::canonicalize(&config_dir_path)
                        .unwrap()
                        .to_string_lossy(),
                    "config.json"
                ),
            );
            error.push_str(None, "'".to_string());
            error.print_data_err()
        } else {
            HashMap::new()
        };

        for (key, config) in config.iter_mut() {
            if let Some(credential) = credentials.get(key) {
                if let Some(api_key) = &credential.api_key {
                    config.api_key = Some(api_key.to_string())
                } else {
                    config.api_key = None
                }
            }
        }

        // get the profile
        if let Some(profile) = profile {
            if let Some(profile) = config.get(profile) {
                Some(profile.to_owned())
            } else {
                let mut error = StyledStr::new();

                error.push_str(Some(Style::Error), "error: ".to_string());
                error.push_str(None, "Profile ".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(Some(Style::Warning), profile.to_string());
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
        } else {
            // otherwise return None
            None
        }
    }
}
