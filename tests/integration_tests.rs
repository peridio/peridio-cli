use assert_cmd::Command;
use indent::indent_by;
use predicates::prelude::PredicateBooleanExt;
use serde_json::Value;
use static_init::dynamic;
use std::collections::HashMap;
use std::env::VarError;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::str;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{env, fs};
use tempfile::{tempdir, NamedTempFile, TempPath};
use uuid::Uuid;

#[test]
fn without_command_usage_is_shown() {
    Command::cargo_bin("peridio-cli")
        .unwrap()
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "Usage: peridio-cli [OPTIONS] <COMMAND>",
        ))
        .stderr(
            predicates::str::contains("-a, --api-key <API_KEY>")
                .and(predicates::str::contains("[env: PERIDIO_API_KEY]")),
        )
        .stderr(
            predicates::str::contains("-b, --base-url <BASE_URL>")
                .and(predicates::str::contains("[env: PERIDIO_BASE_URL=]")),
        )
        .stderr(
            predicates::str::contains("-c, --ca-path <CA_PATH>")
                .and(predicates::str::contains("[env: PERIDIO_CA_PATH=]")),
        )
        .stderr(
            predicates::str::contains("-p, --profile <PROFILE>")
                .and(predicates::str::contains("[env: PERIDIO_PROFILE=]")),
        )
        .stderr(
            predicates::str::contains("-d, --config-directory <CONFIG_DIRECTORY>").and(
                predicates::str::contains("[env: PERIDIO_CONFIG_DIRECTORY=]"),
            ),
        );
}

#[test]
fn with_users_subcommands_are_shown() {
    Command::cargo_bin("peridio-cli")
        .unwrap()
        .arg("users")
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "Usage: peridio-cli users <COMMAND>",
        ))
        .stderr(predicates::str::contains("  me"));
}

#[test]
#[ignore]
fn with_users_with_me_shows_email_and_username() {
    let base_url = base_url();
    let ca_path_buf = peridio_cloud_certificate_authority_path();
    let user = User::create(&format!("{}.com", random_name()));
    let org = Organization::create(&user);
    let api_key = APIKey::create(&org, &user);
    let ca_path = ca_path_buf.into_os_string().into_string().unwrap();

    PERIDIO_CLOUD_API.init();

    let assert = Command::cargo_bin("peridio-cli")
        .unwrap()
        .args(["--base-url", &base_url])
        .args(["--ca-path", &ca_path])
        .args(["--api-key", &api_key])
        .arg("users")
        .arg("me")
        .assert();

    let output = assert.get_output();
    let stdout = str::from_utf8(output.stdout.as_slice()).unwrap();
    let stderr = str::from_utf8(output.stderr.as_slice()).unwrap();

    if !output.status.success() {
        let peridio_cloud_api_exit_code = PERIDIO_CLOUD_API.write().exit_code();

        panic!(
            "peridio-cli --base-url {} --ca-path {} --api-key {}\nSTDOUT:\n  {}\nSTDERR:\n  {}\n\nmix -S phx.server\nEXIT:{}",
            base_url,
            ca_path,
            api_key,
            indent_by(2, stdout),
            indent_by(2, stderr),
            peridio_cloud_api_exit_code
        );
    }

    let user_value: Value = serde_json::from_str(stdout)
        .unwrap_or_else(|error| panic!("{error} in \"{stdout}\"\nSTDERR: {stderr}"));
    let Value::Object(user_map) = user_value else {
        panic!("{user_value} is not a JSON object");
    };
    let Some(data_value) = user_map.get("data") else {
        panic!("{user_map:?} does not have data");
    };
    let Value::Object(data_map) = data_value else {
        panic!("{data_value:?} is not an object");
    };

    let Some(email_value) = data_map.get("email") else {
        panic!("data object ({data_map:?}) does not have email")
    };
    let Value::String(email_string) = email_value else {
        panic!("email ({email_value:?}) is not a string");
    };
    assert_eq!(email_string, &user.email);

    let Some(username_value) = data_map.get("username") else {
        panic!("data object ({data_map:?}) does not have username");
    };
    let Value::String(username_string) = username_value else {
        panic!("username ({username_value:?}) is not a string");
    };
    assert_eq!(username_string, &user.username);
}

fn base_url() -> String {
    format!("https://{HOST}:{PORT}")
}

fn base_address() -> SocketAddr {
    (HOST, PORT).to_socket_addrs().unwrap().next().unwrap()
}

fn peridio_cloud_certificate_authority_path() -> PathBuf {
    peridio_cloud_certificate_authority_directory_path().join("ca.pem")
}

fn peridio_cloud_certificate_authority_directory_path() -> PathBuf {
    // See https://github.com/peridio/peridio-cloud/blob/9f94887fb374f5b781cc1d37e1b8a4b59738456f/config/dev.exs#L7-L9
    match env::var("peridio_cloud_CA_DIR") {
        Ok(peridio_cloud_ca_dir) => PathBuf::from(peridio_cloud_ca_dir),
        Err(var_error) => match var_error {
            VarError::NotPresent => peridio_cloud_path().join("test/fixtures/ssl"),
            VarError::NotUnicode(_) => {
                panic!("peridio_cloud_CA_DIR environment variable is not valid unicode")
            }
        },
    }
}

fn peridio_cloud_path() -> PathBuf {
    let path = match env::var("PERIDIO_CLOUD_PATH") {
        Ok(peridio_cloud_path) => PathBuf::from(peridio_cloud_path),
        Err(var_error) => match var_error {
            VarError::NotPresent => fs::canonicalize("../peridio-cloud").unwrap(),
            VarError::NotUnicode(_) => {
                panic!("PERIDIO_CLOUD_PATH environment variable is not valid unicode")
            }
        },
    };

    if path.is_dir() {
        path
    } else {
        panic!(
            "peridio_cloud path ({}) is not a directory",
            path.to_str().unwrap()
        );
    }
}

fn random_name() -> String {
    format!(
        "peridio-cli-{}",
        Uuid::now_v7()
            .hyphenated()
            .encode_upper(&mut Uuid::encode_buffer())
    )
}

struct APIKey {}

impl APIKey {
    fn create(org: &Organization, user: &User) -> String {
        let description = random_name();
        let api_key_temp_path = NamedTempFile::new().unwrap().into_temp_path();

        match peridio_cloud_core_mix_command()
            .arg("peridio.api_key.create")
            .args(["--description", &description])
            .args(["--org-prn-file", org.prn_temp_path.to_str().unwrap()])
            .args(["--user-prn-file", user.prn_temp_path.to_str().unwrap()])
            .args(["--api-key-file", api_key_temp_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    panic!(
                        "mix peridio.api_key.create failed\n  STDOUT:\n    {}\n\n  STDERR:\n    {}",
                        indent_by(4, String::from_utf8(output.stdout).unwrap()),
                        indent_by(4, String::from_utf8(output.stderr).unwrap())
                    );
                } else {
                    String::from_utf8(std::fs::read(api_key_temp_path).unwrap()).unwrap()
                }
            }
            Err(error) => panic!("{error:?}"),
        }
    }
}

struct Organization {
    prn_temp_path: TempPath,
}

impl Organization {
    fn create(user: &User) -> Self {
        let name = random_name();
        let prn_temp_path = NamedTempFile::new().unwrap().into_temp_path();

        match peridio_cloud_core_mix_command()
            .arg("peridio.org.create")
            .args(["--name", &name])
            .args(["--prn-file", prn_temp_path.to_str().unwrap()])
            .args(["--user-prn-file", user.prn_temp_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    panic!(
                        "mix peridio.org.create failed\n  STDOUT:\n    {}\n\n  STDERR:\n    {}",
                        indent_by(4, String::from_utf8(output.stdout).unwrap()),
                        indent_by(4, String::from_utf8(output.stderr).unwrap())
                    );
                }
            }
            Err(error) => panic!("{error:?}"),
        }

        Self { prn_temp_path }
    }
}

struct User {
    username: String,
    email: String,
    prn_temp_path: TempPath,
}

impl User {
    fn create(email_domain: &str) -> Self {
        let username = random_name();
        let email = format!("{username}@{email_domain}");
        let password = "peridio-cli";
        let prn_temp_path = NamedTempFile::new().unwrap().into_temp_path();

        match peridio_cloud_core_mix_command()
            .arg("peridio.user.create")
            .args(["--username", username.as_str()])
            .args(["--email", email.as_str()])
            .args(["--password", password])
            .args(["--prn-file", prn_temp_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    panic!(
                        "mix peridio.user.create failed\n  STDOUT:\n    {}\n\n  STDERR:\n    {}",
                        indent_by(4, String::from_utf8(output.stdout).unwrap()),
                        indent_by(4, String::from_utf8(output.stderr).unwrap())
                    );
                }
            }
            Err(error) => panic!("{error:?}"),
        }

        Self {
            username,
            email,
            prn_temp_path,
        }
    }
}

fn peridio_cloud_api_mix_command() -> std::process::Command {
    let mut command = mix_command();
    command.args(["do", "--app", "peridio_cloud_api"]);

    command
}

fn peridio_cloud_core_mix_command() -> std::process::Command {
    let mut command = mix_command();
    command.args(["do", "--app", "peridio_cloud_core"]);

    command
}

fn mix_command() -> std::process::Command {
    let mut command = std::process::Command::new("mix");

    require_env_var("DATABASE_URL");

    command
        .current_dir(peridio_cloud_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

fn require_env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} is not set"))
}

const HOST: &str = "api.test.peridio.com";
const PORT: u16 = 4002;
#[dynamic(lazy, drop)]
static mut PERIDIO_CLOUD_API: PeridioCloudAPI = PeridioCloudAPI::new();

struct PeridioCloudAPI {
    child: Option<Child>,
}
impl PeridioCloudAPI {
    fn new() -> Self {
        Self {
            child: Some(Self::child()),
        }
    }

    fn child() -> Child {
        match peridio_cloud_api_mix_command()
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .args(["phx.server"])
            .spawn()
        {
            Ok(child) => {
                Self::wait_for_server();

                child
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => panic!("mix not found"),
                _ => panic!("{error:?}"),
            },
        }
    }

    fn wait_for_server() {
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(30) {
            match TcpStream::connect_timeout(&base_address(), Duration::from_secs(1)) {
                Ok(_) => break,
                Err(error) => match error.kind() {
                    ErrorKind::ConnectionRefused => sleep(Duration::from_secs(5)),
                    _ => panic!("{error:?}"),
                },
            }
        }
    }

    fn exit_code(&mut self) -> i32 {
        match self.child.take() {
            Some(mut child) => match child.try_wait().unwrap() {
                Some(exit_status) => exit_status.code().unwrap_or(0),
                None => {
                    child.kill().unwrap();

                    0
                }
            },
            None => 0,
        }
    }
}
impl Drop for PeridioCloudAPI {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if let Err(error) = child.kill() {
                eprintln!("Could not kill child process: {error}")
            }
        }
    }
}

#[test]
fn config_init_creates_profile() {
    // Create a temporary directory for config files
    let temp_dir = tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_str().unwrap().to_string();

    // Prepare test values
    let profile_name = "test-profile";
    let api_key = "test-api-key";

    // Run config init with simulated input
    let mut cmd = Command::cargo_bin("peridio-cli").unwrap();
    cmd.args(["--profile", "temp-profile"]) // We need to provide a profile when using config-directory
        .args(["--config-directory", &temp_dir_path])
        .args(["config", "profiles", "create"])
        .args(["--name", profile_name])
        .args(["--api-key", api_key])
        .args(["--no-input"])
        .assert()
        .success()
        .stderr(predicates::str::contains(format!(
            "Profile '{profile_name}' configured successfully"
        )));

    // Verify config.json file
    let config_path = temp_dir.path().join("config.json");
    assert!(config_path.exists(), "config.json wasn't created");

    let config_content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();

    assert_eq!(config["version"], 2);
    assert!(config["profiles"].is_object());
    assert!(config["profiles"][profile_name].is_object());

    // Verify api_version defaults to 2
    assert_eq!(config["profiles"][profile_name]["api_version"], 2);

    // Verify credentials.json file
    let creds_path = temp_dir.path().join("credentials.json");
    assert!(creds_path.exists(), "credentials.json wasn't created");

    let creds_content = fs::read_to_string(&creds_path).unwrap();
    let creds: HashMap<String, serde_json::Value> = serde_json::from_str(&creds_content).unwrap();

    assert!(creds.contains_key(profile_name));
    assert_eq!(creds[profile_name]["api_key"], api_key);
}

#[test]
fn config_upgrade_sets_api_version() {
    // Create a temporary directory for config files
    let temp_dir = tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_str().unwrap().to_string();

    // Create a v1 config file (without api_version)
    let config_path = temp_dir.path().join("config.json");
    let v1_config = serde_json::json!({
        "test-profile": {
            "api_key": "test-key",
            "base_url": "https://example.com",
            "ca_path": "/path/to/ca.pem",
            "organization_name": "test-org"
        }
    });
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&v1_config).unwrap(),
    )
    .unwrap();

    // Run config upgrade (need to provide dummy profile due to clap constraint)
    let mut cmd = Command::cargo_bin("peridio-cli").unwrap();
    cmd.args(["--profile", "dummy-profile"])
        .args(["--config-directory", &temp_dir_path])
        .args(["config", "upgrade"])
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "The config file has been migrated to v2",
        ));

    // Verify the upgraded config has api_version set to 2
    let config_content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();

    assert_eq!(config["version"], 2);
    assert!(config["profiles"]["test-profile"]["api_version"].is_number());
    assert_eq!(config["profiles"]["test-profile"]["api_version"], 2);
}
