use std::env::VarError;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::str;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{env, fs};

use assert_cmd::Command;
use indent::indent_by;
use predicates::prelude::PredicateBooleanExt;
use serde_json::Value;
use static_init::dynamic;
use tempfile::{NamedTempFile, TempPath};
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
            predicates::str::contains("-o, --organization-name <ORGANIZATION_NAME>").and(
                predicates::str::contains("[env: PERIDIO_ORGANIZATION_NAME=]"),
            ),
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
fn with_users_with_me_shows_email_and_username() {
    let base_url = base_url();
    let ca_path_buf = peridio_cloud_certificate_authority_path();
    let user = User::create(&format!("{}.com", random_name()));
    let api_key = user.create_api_key();
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
        .unwrap_or_else(|error| panic!("{} in \"{}\"\nSTDERR: {}", error, stdout, stderr));
    let Value::Object(user_map) = user_value else {
        panic!("{} is not a JSON object", user_value);
    };
    let Some(data_value) = user_map.get("data") else {
        panic!("{:?} does not have data", user_map);
    };
    let Value::Object(data_map) = data_value else {
        panic!("{:?} is not an object", data_value);
    };

    let Some(email_value) = data_map.get("email") else {
        panic!("data object ({:?}) does not have email", data_map)
    };
    let Value::String(email_string) = email_value else {
        panic!("email ({:?}) is not a string", email_value);
    };
    assert_eq!(email_string, &user.email);

    let Some(username_value) = data_map.get("username") else {
        panic!("data object ({:?}) does not have username", data_map);
    };
    let Value::String(username_string) = username_value else {
        panic!("username ({:?}) is not a string", username_value);
    };
    assert_eq!(username_string, &user.username);
}

fn base_url() -> String {
    format!("https://{}:{}", HOST, PORT)
}

fn base_address() -> SocketAddr {
    (HOST, PORT).to_socket_addrs().unwrap().next().unwrap()
}

fn peridio_cloud_api_path() -> PathBuf {
    let path = peridio_cloud_path().join("apps/peridio_cloud_api");

    if path.is_dir() {
        path
    } else {
        panic!(
            "peridio_cloud_api path ({}) is not a directory",
            path.to_str().unwrap()
        )
    }
}

fn peridio_cloud_core_path() -> PathBuf {
    let path = peridio_cloud_path().join("apps/peridio_cloud_core");

    if path.is_dir() {
        path
    } else {
        panic!(
            "peridio_cloud_core path ({}) is not a directory",
            path.to_str().unwrap()
        )
    }
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

struct User {
    username: String,
    email: String,
    prn_temp_path: TempPath,
}
impl User {
    fn create(email_domain: &str) -> Self {
        let username = random_name();
        let email = format!("{}@{}", username, email_domain);
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
            Err(error) => panic!("{:?}", error),
        }

        Self {
            username,
            email,
            prn_temp_path,
        }
    }

    fn create_api_key(&self) -> String {
        let note = random_name();
        let user_token_temp_path = NamedTempFile::new().unwrap().into_temp_path();

        match peridio_cloud_core_mix_command()
            .arg("peridio.user_token.create")
            .args(["--user-prn-file", self.prn_temp_path.to_str().unwrap()])
            .args(["--note", &note])
            .args(["--user-token-file", user_token_temp_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    panic!(
                        "mix peridio.user_token.create failed\n  STDOUT:\n    {}\n\n  STDERR:\n    {}",
                        indent_by(4, String::from_utf8(output.stdout).unwrap()),
                        indent_by(4, String::from_utf8(output.stderr).unwrap())
                    );
                } else {
                    String::from_utf8(std::fs::read(user_token_temp_path).unwrap()).unwrap()
                }
            }
            Err(error) => panic!("{:?}", error),
        }
    }
}

fn peridio_cloud_api_mix_command() -> std::process::Command {
    let mut command = mix_command();
    command.current_dir(peridio_cloud_api_path());

    command
}

fn peridio_cloud_core_mix_command() -> std::process::Command {
    let mut command = mix_command();
    command.current_dir(peridio_cloud_core_path());

    command
}

fn mix_command() -> std::process::Command {
    let mut command = std::process::Command::new("mix");

    require_env_var("DATABASE_URL");

    command
        .current_dir(peridio_cloud_api_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

fn require_env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{} is not set", name))
}

const HOST: &'static str = "api.test.peridio.com";
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
                _ => panic!("{:?}", error),
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
                    _ => panic!("{:?}", error),
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
                eprintln!("Could not kill child process: {}", error)
            }
        }
    }
}
