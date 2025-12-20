pub mod list;
pub mod prn;
pub mod serde_introspection;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use directories::ProjectDirs;
use dirs::home_dir;
use serde_json::{Map, Value};
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::{env, path::PathBuf};
use termcolor::WriteColor;
use uuid::Uuid;

use crate::GlobalOptions;

pub struct StyledStr {
    messages: Vec<(Option<Style>, String)>,
}

impl StyledStr {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn push_str(&mut self, style: Option<Style>, msg: String) {
        if !msg.is_empty() {
            self.messages.push((style, msg));
        }
    }

    pub fn print_msg(&self) -> std::io::Result<()> {
        let bufwtr = termcolor::BufferWriter::stderr(termcolor::ColorChoice::Always);
        let mut buffer = bufwtr.buffer();

        for (style, message) in &self.messages {
            let mut color = termcolor::ColorSpec::new();
            match style {
                Some(Style::Success) => {
                    color.set_fg(Some(termcolor::Color::Green));
                }
                Some(Style::Warning) => {
                    color.set_fg(Some(termcolor::Color::Yellow));
                }
                Some(Style::Error) => {
                    color.set_fg(Some(termcolor::Color::Red));
                    color.set_bold(true);
                }
                None => {}
            }

            buffer.set_color(&color)?;
            write!(buffer, "{message}")?;
        }

        write!(buffer, "\r\n")?;
        bufwtr.print(&buffer)?;

        Ok(())
    }

    pub fn print_data_err(&self) -> ! {
        self.print_msg().unwrap();

        // DATAERR
        std::process::exit(65)
    }

    pub fn print_success(&self) -> ! {
        self.print_msg().unwrap();

        // SUCCESS
        std::process::exit(0)
    }
}

pub enum Style {
    Success,
    Warning,
    Error,
}

pub fn maybe_json(data: Option<String>) -> Option<Map<String, Value>> {
    if let Some(json) = data {
        serde_json::from_str(json.as_str()).ok()
    } else {
        None
    }
}

pub fn maybe_config_directory(global_options: &GlobalOptions) -> Option<PathBuf> {
    if let Some(config_dir) = &global_options.config_directory {
        let config_dir_path = PathBuf::from(config_dir);

        if config_dir_path.exists() {
            // use this config
            Some(config_dir_path)
        } else {
            None
        }
    } else if let Some(proj_dirs) = ProjectDirs::from("", "", "peridio") {
        let cache_dir = proj_dirs.config_dir();

        Some(cache_dir.to_path_buf())
    } else {
        None
    }
}

fn prn_error(cmd: &clap::Command, arg: Option<&clap::Arg>, error: &str) -> clap::Error {
    let mut err = clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
    if let Some(arg) = arg {
        err.insert(
            ContextKind::InvalidArg,
            ContextValue::String(arg.to_string()),
        );
    }
    err.insert(
        ContextKind::InvalidValue,
        ContextValue::String(error.to_string()),
    );
    err
}

#[derive(Clone, PartialEq, Debug)]
pub enum PRNType {
    APIKey,
    Artifact,
    ArtifactVersion,
    AuditLog,
    Binary,
    BinaryPart,
    BinarySignature,
    Bundle,
    BundleOverride,
    CACertificate,
    Cohort,
    Deployment,
    Device,
    DeviceCertificate,
    Event,
    Firmware,
    OrgUser,
    Organization,
    Product,
    Release,
    ReleaseClaim,
    SigningKey,
    Tunnel,
    User,
    WebConsoleShell,
    Webhook,
}

impl TryFrom<String> for PRNType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<PRNType, Self::Error> {
        match value.as_str() {
            "api_key" => Ok(Self::APIKey),
            "artifact" => Ok(Self::Artifact),
            "artifact_version" => Ok(Self::ArtifactVersion),
            "audit_log" => Ok(Self::AuditLog),
            "binary" => Ok(Self::Binary),
            "binary_part" => Ok(Self::BinaryPart),
            "binary_signature" => Ok(Self::BinarySignature),
            "bundle" => Ok(Self::Bundle),
            "bundle_override" => Ok(Self::BundleOverride),
            "ca_certificate" => Ok(Self::CACertificate),
            "cohort" => Ok(Self::Cohort),
            "deployment" => Ok(Self::Deployment),
            "device" => Ok(Self::Device),
            "device_certificate" => Ok(Self::DeviceCertificate),
            "event" => Ok(Self::Event),
            "firmware" => Ok(Self::Firmware),
            "org_user" => Ok(Self::OrgUser),
            "organization" => Ok(Self::Organization),
            "product" => Ok(Self::Product),
            "release" => Ok(Self::Release),
            "release_claim" => Ok(Self::ReleaseClaim),
            "signing_key" => Ok(Self::SigningKey),
            "tunnel" => Ok(Self::Tunnel),
            "user" => Ok(Self::User),
            "web_console_shell" => Ok(Self::WebConsoleShell),
            "webhook" => Ok(Self::Webhook),
            _ => Err("Invalid PRN type"),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct PRNValueParser(PRNType);

impl PRNValueParser {
    pub fn new(prn_type: PRNType) -> Self {
        Self(prn_type)
    }
}

impl clap::builder::TypedValueParser for PRNValueParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value: String = value.to_str().unwrap().to_owned();

        let mut split = value.split(':').fuse();

        let prn_length = split.clone().count();

        if !(3..=5).contains(&prn_length) {
            return Err(prn_error(cmd, arg, "Invalid PRN"));
        }

        if split.next().is_some_and(|x| x != "prn") {
            return Err(prn_error(cmd, arg, "Invalid PRN"));
        }

        if split.next().is_some_and(|x| x != "1") {
            return Err(prn_error(cmd, arg, "Invalid PRN"));
        }

        match prn_length {
            3 => {
                // organization prn only
                if self.0 != PRNType::Organization {
                    return Err(prn_error(cmd, arg, "Invalid PRN type"));
                }
                // the uuid has to be valid
                if Uuid::try_parse(split.next().unwrap()).is_err() {
                    return Err(prn_error(
                        cmd,
                        arg,
                        "Invalid PRN UUID, expected 'organization' UUID in PRN",
                    ));
                }

                0
            }
            4 => {
                // user
                if self.0 != PRNType::User {
                    return Err(prn_error(cmd, arg, "Invalid PRN type"));
                }

                let prn_type = PRNType::try_from(split.next().unwrap().to_string());

                if prn_type.is_err() {
                    return Err(prn_error(cmd, arg, "Invalid PRN type"));
                }

                let prn_type = prn_type.unwrap();

                if prn_type != PRNType::User {
                    return Err(prn_error(cmd, arg, "Invalid PRN type, expected 'user'"));
                }

                0
            }
            5 => {
                // the org uuid has to be valid
                if Uuid::try_parse(split.next().unwrap()).is_err() {
                    return Err(prn_error(
                        cmd,
                        arg,
                        "Invalid PRN UUID, expected valid UUID in PRN",
                    ));
                }

                let prn_type = PRNType::try_from(split.next().unwrap().to_string());

                if prn_type.is_err() {
                    return Err(prn_error(cmd, arg, "Invalid PRN type"));
                }

                let prn_type = prn_type.unwrap();

                if self.0 != prn_type {
                    return Err(prn_error(
                        cmd,
                        arg,
                        format!("Invalid PRN type, expected '{:#?}' PRN", self.0).as_str(),
                    ));
                }

                // the uuid has to be valid
                if Uuid::try_parse(split.next().unwrap()).is_err() {
                    return Err(prn_error(
                        cmd,
                        arg,
                        "Invalid PRN UUID, expected valid UUID in PRN",
                    ));
                }

                0
            }
            _ => return Err(prn_error(cmd, arg, "Invalid PRN")),
        };

        Ok(value)
    }
}

/// Build a usable path from user input which may be:
/// - absolute (starts with /)
/// - relative to home (starts with ~)
/// - relative to the current directory
///
/// Returns an error if path expansion fails.
pub fn path_from(input: &str) -> Result<PathBuf, std::io::Error> {
    if input.starts_with('/') {
        // Absolute path
        Ok(PathBuf::from(input))
    } else if input.starts_with("~/") || input == "~" {
        // Home directory expansion
        match home_dir() {
            Some(mut path) => {
                if input.len() > 1 {
                    path.push(&input[2..]);
                }
                Ok(normalize_path(path))
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Home directory not found",
            )),
        }
    } else {
        // Relative path to current directory
        let base = env::current_dir()?;
        let path = base.join(input);
        Ok(normalize_path(path))
    }
}

/// Normalize a path by resolving parent directory references (..)
///
/// This handles paths like "a/b/../c" -> "a/c", but doesn't resolve symlinks.
/// Preserves trailing slashes in the original path.
pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path_ref = path.as_ref();
    let ends_with_slash = path_ref
        .to_str()
        .map_or_else(|| false, |s| s.ends_with('/'));

    let mut normalized = PathBuf::new();
    for component in path_ref.components() {
        match component {
            Component::ParentDir => {
                // Only pop if we have something to pop, otherwise keep the ..
                if !normalized.as_os_str().is_empty() && normalized.file_name().is_some() {
                    normalized.pop();
                } else {
                    normalized.push(component);
                }
            }
            _ => {
                normalized.push(component);
            }
        }
    }

    if ends_with_slash {
        normalized.push("");
    }

    normalized
}

pub use prn::PRNBuilder;

// #[derive(Clone, Debug)]
// pub enum ExpandResult {
//     All,
//     Fields(Vec<String>),
// }

// #[derive(Clone, Debug)]
// pub struct ExpandValueParser<E: DeserializeOwned + Clone + Send + Sync + 'static>(
//     std::marker::PhantomData<E>,
// );

// impl<E: DeserializeOwned + Clone + Send + Sync + 'static> ExpandValueParser<E> {
//     pub fn new() -> Self {
//         let phantom: std::marker::PhantomData<E> = Default::default();
//         Self(phantom)
//     }
// }

// impl<E: DeserializeOwned + Clone + Send + Sync + 'static> TypedValueParser
//     for ExpandValueParser<E>
// {
//     type Value = ExpandResult;

//     fn parse_ref(
//         &self,
//         cmd: &clap::Command,
//         arg: Option<&clap::Arg>,
//         value: &std::ffi::OsStr,
//     ) -> Result<Self::Value, clap::Error> {
//         let value = value.to_str().unwrap();

//         let result = match value {
//             "" => ExpandResult::All,
//             value => {
//                 let values: HashSet<String> = value.split(',').map(|x| x.to_string()).collect();

//                 let struct_fields: HashSet<String> = serde_introspect::<E>()
//                     .iter()
//                     .map(|x| x.to_string())
//                     .collect();

//                 // a provided field is not part of the struct
//                 if !values.is_subset(&struct_fields) {
//                     let diff: Vec<String> = struct_fields
//                         .difference(&values)
//                         .map(|x| x.to_string())
//                         .collect();
//                     let error_fields = diff.join(",");

//                     let mut err = clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
//                     if let Some(arg) = arg {
//                         err.insert(
//                             ContextKind::InvalidArg,
//                             ContextValue::String(arg.to_string()),
//                         );
//                     }
//                     err.insert(
//                         ContextKind::InvalidValue,
//                         ContextValue::String(error_fields),
//                     );
//                     return Err(err);
//                 }

//                 let result = values.into_iter().collect();

//                 ExpandResult::Fields(result)
//             }
//         };

//         Ok(result)
//     }
// }
