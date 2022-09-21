use std::{
    cmp::min,
    env,
    fs::{create_dir_all, rename},
    io::{Cursor, ErrorKind, Seek, SeekFrom, Write},
    path::Path,
};

use directories::ProjectDirs;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::ClientBuilder;
use serde::Deserialize;
use structopt::StructOpt;
use tar::Archive;

use crate::Error;

#[derive(Deserialize, Debug)]
struct GithubAssetResponse {
    browser_download_url: String,
}

#[derive(Deserialize, Debug)]
struct GithubResponse {
    tag_name: String,
    assets: Vec<GithubAssetResponse>,
}

#[derive(StructOpt, Debug)]
pub enum UpgradeCommand {
    Upgrade(DoUpgradeCommand),
}

impl UpgradeCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Upgrade(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct DoUpgradeCommand {}

impl DoUpgradeCommand {
    async fn run(self) -> Result<(), Error> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "peridio", "peridio cli") {
            let cache_dir = proj_dirs.cache_dir();

            create_dir_all(cache_dir).unwrap();

            if let Ok(resp) = Self::get_latest_release_info().await {
                let current_version = env!("CARGO_PKG_VERSION");
                let github_asset_info = resp.assets.first().unwrap();

                // no need to update
                if resp.tag_name == current_version {
                    println!("CLI already up to date");
                    return Ok(());
                }

                if let Err(message) = Self::download_update(cache_dir, github_asset_info).await {
                    println!("{}", message);
                    return Ok(());
                }

                if let Err(message) = Self::apply_update(cache_dir, &resp) {
                    println!("{}", message);
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    fn apply_update(path: &Path, github_response: &GithubResponse) -> Result<(), String> {
        let update_file = path.join("peridio");

        let current_cli_executable =
            env::current_exe().map_err(|_| "Can't retrieve the current cli directory")?;

        match rename(update_file, &current_cli_executable) {
            Err(err) => {
                if err.kind() == ErrorKind::PermissionDenied {
                    return Err(format!(
                        "CLI failed to upgrade: permission denied writing to {}",
                        &current_cli_executable.display()
                    ));
                }
                return Err(format!(
                    "CLI failed to upgrade: unknown error writing to {}",
                    &current_cli_executable.display()
                ));
            }
            Ok(_) => {}
        }

        println!("CLI upgraded successfully ({})", github_response.tag_name);

        Ok(())
    }

    async fn download_update(
        download_path: &Path,
        github_asset_info: &GithubAssetResponse,
    ) -> Result<(), String> {
        let client = ClientBuilder::new().use_rustls_tls().build().unwrap();
        let url = &github_asset_info.browser_download_url;

        // Reqwest setup
        let res = client
            .get(url)
            .send()
            .await
            .map_err(|_| format!("Failed to GET from '{}'", url))?;

        let total_size = res
            .content_length()
            .ok_or(format!("Failed to get content length from '{}'", url))?;

        // Indicatif setup
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::with_template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("#>-"));

        // download chunks
        let mut downloaded: u64 = 0;

        let mut stream = res.bytes_stream();

        let mem = Vec::new();
        let mut buff = Cursor::new(mem);

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|_| "Error while downloading file".to_string())?;
            buff.write_all(&chunk)
                .map_err(|_| "Error while writing to buffer".to_string())?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
        }

        pb.finish_and_clear();

        buff.seek(SeekFrom::Start(0)).unwrap();

        let gz = GzDecoder::new(&mut buff);

        let mut archive = Archive::new(gz);

        archive
            .unpack(&download_path)
            .map_err(|_| "Error while saving the updated file".to_string())?;

        Ok(())
    }

    async fn get_latest_release_info() -> Result<GithubResponse, reqwest::Error> {
        let client = ClientBuilder::new().use_rustls_tls().build().unwrap();

        client
            .get("https://api.github.com/repos/peridio/morel/releases/latest")
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "peridio/morel")
            .send()
            .await?
            .json::<GithubResponse>()
            .await
    }
}
