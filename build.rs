use std::env;
use std::process::Command;

fn main() {
    // This is ran only during build process, we expect to always have git when building the app
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!(
        "cargo:rustc-env=PERIDIO_CLI_VERSION={} {}",
        env!("CARGO_PKG_VERSION"),
        git_hash
    );
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
}
