use crate::config::config_v1::ConfigV1;
use crate::config::config_v1::ProfileV1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::PathBuf;

pub type ProfileNameV2 = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileV2 {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub ca_path: Option<String>,
}

impl TryFrom<ProfileV1> for ProfileV2 {
    type Error = &'static str;

    fn try_from(profile_v1: ProfileV1) -> Result<Self, Self::Error> {
        let profile_v2 = ProfileV2 {
            api_key: profile_v1.api_key,
            base_url: profile_v1.base_url,
            ca_path: profile_v1.ca_path,
        };
        Ok(profile_v2)
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProfilesV2(HashMap<ProfileNameV2, ProfileV2>);

impl Deref for ProfilesV2 {
    type Target = HashMap<ProfileNameV2, ProfileV2>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ProfilesV2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<(String, ProfileV2)> for ProfilesV2 {
    fn from_iter<I: IntoIterator<Item = (String, ProfileV2)>>(iter: I) -> Self {
        let mut c = ProfilesV2::default();

        for (k, v) in iter {
            c.insert(k, v);
        }

        c
    }
}

impl TryFrom<ConfigV1> for ProfilesV2 {
    type Error = &'static str;

    fn try_from(config_v1: ConfigV1) -> Result<Self, Self::Error> {
        let profiles_v2: Result<ProfilesV2, _> = config_v1
            .iter()
            .map(
                |(name, profile_v1)| match ProfileV2::try_from(profile_v1.clone()) {
                    Ok(profile_v2) => Ok((name.to_owned(), profile_v2)),
                    Err(_) => Err("atlantis"),
                },
            )
            .collect();
        profiles_v2
    }
}

pub type SigningKeyPairNameV2 = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SigningKeyPairV2 {
    pub signing_key_prn: String,
    pub signing_key_private_path: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct SigningKeyPairsV2(HashMap<SigningKeyPairNameV2, SigningKeyPairV2>);

impl Deref for SigningKeyPairsV2 {
    type Target = HashMap<SigningKeyPairNameV2, SigningKeyPairV2>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SigningKeyPairsV2 {
    fn deref_mut(&mut self) -> &mut HashMap<SigningKeyPairNameV2, SigningKeyPairV2> {
        &mut self.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CertificateAuthorityV2 {
    pub private_key: String,
    pub certificate: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct CertificateAuthoritiesV2(HashMap<String, CertificateAuthorityV2>);

impl Deref for CertificateAuthoritiesV2 {
    type Target = HashMap<String, CertificateAuthorityV2>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigV2 {
    pub version: u8,
    pub profiles: ProfilesV2,
    pub signing_key_pairs: Option<SigningKeyPairsV2>,
    pub certificate_authorities: Option<CertificateAuthoritiesV2>,
}

impl Default for ConfigV2 {
    fn default() -> ConfigV2 {
        ConfigV2 {
            version: 2,
            profiles: ProfilesV2::default(),
            signing_key_pairs: Some(SigningKeyPairsV2::default()),
            certificate_authorities: Some(CertificateAuthoritiesV2::default()),
        }
    }
}

impl TryFrom<ConfigV1> for ConfigV2 {
    type Error = &'static str;

    fn try_from(config_v1: ConfigV1) -> Result<Self, Self::Error> {
        match ProfilesV2::try_from(config_v1) {
            Ok(profiles_v2) => {
                let config_v2 = ConfigV2 {
                    version: 1,
                    profiles: profiles_v2,
                    signing_key_pairs: Some(SigningKeyPairsV2::default()),
                    certificate_authorities: Some(CertificateAuthoritiesV2::default()),
                };
                Ok(config_v2)
            }
            Err(err) => Err(err),
        }
    }
}

impl TryFrom<&PathBuf> for ConfigV2 {
    type Error = Box<dyn std::error::Error>;

    fn try_from(config_path: &PathBuf) -> Result<Self, Self::Error> {
        let config_data = fs::read_to_string(config_path)?;
        Ok(serde_json::from_str::<ConfigV2>(&config_data)?)
    }
}
