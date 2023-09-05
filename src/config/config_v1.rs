use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;

pub type ProfileNameV1 = String;

#[derive(Serialize, Deserialize, Clone)]
pub struct ProfilesV1(HashMap<ProfileNameV1, ProfileV1>);

impl Deref for ProfilesV1 {
    type Target = HashMap<ProfileNameV1, ProfileV1>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProfileV1 {
    #[serde(default)]
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub ca_path: Option<String>,
    pub organization_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigV1(ProfilesV1);

impl Deref for ConfigV1 {
    type Target = ProfilesV1;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
