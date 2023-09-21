use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use crate::NonExistingPathSnafu;
use backon::ExponentialBuilder;
use backon::Retryable;
use base64::engine::general_purpose;
use base64::Engine;
use clap::Parser;

use console::style;
use futures_util::stream;
use futures_util::StreamExt;
use indicatif::ProgressBar;
use indicatif::ProgressState;
use indicatif::ProgressStyle;
use peridio_sdk::api::binaries::Binary;
use peridio_sdk::api::binaries::BinaryState;
use peridio_sdk::api::binaries::CreateBinaryParams;
use peridio_sdk::api::binaries::CreateBinaryResponse;
use peridio_sdk::api::binaries::GetBinaryParams;
use peridio_sdk::api::binaries::GetBinaryResponse;
use peridio_sdk::api::binaries::ListBinariesParams;
use peridio_sdk::api::binaries::ListBinariesResponse;
use peridio_sdk::api::binaries::UpdateBinaryParams;
use peridio_sdk::api::binaries::UpdateBinaryResponse;
use peridio_sdk::api::binary_parts::ListBinaryPart;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use reqwest::Body;
use reqwest::Client;
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use std::io::Read;
use std::io::Seek;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, io};

#[derive(Parser, Debug)]
pub enum BinariesCommand {
    Create(Command<CreateCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
}

impl BinariesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]

pub struct CreateCommand {
    #[arg(long)]
    artifact_version_prn: String,
    #[arg(long)]
    description: Option<String>,
    #[arg(
        long,
        conflicts_with = "content_path",
        required_unless_present = "content_path"
    )]
    hash: Option<String>,
    #[arg(
        long,
        conflicts_with = "content_path",
        required_unless_present = "content_path"
    )]
    size: Option<u64>,
    #[arg(long)]
    target: String,
    #[arg(
        long,
        conflicts_with_all = ["hash", "size"],
        required_unless_present_any = ["hash", "size"]
    )]
    content_path: Option<String>,
    #[arg(
        long,
        requires = "content_path",
        default_value = "5242880",
        value_parser = clap::value_parser!(u64).range(5242880..50000000000)
    )]
    binary_part_size: Option<u64>,
    #[arg(long, requires = "content_path")]
    concurrency: Option<u8>,
    #[arg(
        long,
        short = 's',
        conflicts_with = "signing_key_private",
        conflicts_with = "signing_key_prn",
        required_unless_present_any = ["signing_key_private", "signing_key_prn"],
        help = "The name of a signing key pair as defined in your Peridio CLI config."
    )]
    signing_key_pair: Option<String>,
    #[arg(
        long,
        conflicts_with = "signing_key_pair",
        required_unless_present = "signing_key_pair",
        requires = "signing_key_prn",
        help = "The PEM base64-encoded PKCS #8 private key."
    )]
    signing_key_private: Option<String>,
    #[arg(
        long,
        conflicts_with = "signing_key_pair",
        required_unless_present = "signing_key_pair",
        requires = "signing_key_private",
        help = "The PRN of the signing key to tell Peridio to verify the signature with."
    )]
    signing_key_prn: Option<String>,
    #[arg(
        long,
        default_value = "false",
        conflicts_with = "concurrency",
        conflicts_with = "binary_part_size",
        conflicts_with = "signing_key_pair"
    )]
    skip_upload: bool,
}

impl CreateCommand {
    async fn run(
        &self,
        global_options: GlobalOptions,
    ) -> Result<Option<CreateBinaryResponse>, Error> {
        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.clone().unwrap(),
            endpoint: global_options.base_url.clone(),
            ca_bundle_path: global_options.ca_path.clone(),
        });

        let binary = match self.get_or_create_binary(&api).await? {
            Some(CreateBinaryResponse { binary }) => {
                if self.skip_upload {
                    return Ok(Some(CreateBinaryResponse { binary }));
                }

                let binary = self.process_binary(&binary, &api, &global_options).await?;

                Some(CreateBinaryResponse { binary })
            }
            None => None,
        };

        Ok(binary)
    }

    async fn process_binary(
        &self,
        binary: &Binary,
        api: &Api,
        global_options: &GlobalOptions,
    ) -> Result<Binary, Error> {
        if matches!(binary.state, BinaryState::Uploadable) {
            self.process_binary_parts(binary, api, global_options)
                .await
                .unwrap();

            println!("{} Start Binary Hashing...", style("[4]").bold().dim());
            // we created the binary parts not move it to hashable
            let binary = self
                .change_binary_status(BinaryState::Hashable, binary, api, global_options)
                .await
                .unwrap();

            // move to hashing
            let binary = self
                .change_binary_status(BinaryState::Hashing, &binary, api, global_options)
                .await
                .unwrap();

            // do signing if available
            if self.signing_key_pair.is_some() || self.signing_key_private.is_some() {
                // wait for hashing to be signable
                println!("{} Waiting Binary to hash...", style("[5]").bold().dim());
                let binary = (|| async {
                    self.check_for_state_change(&binary, api, global_options)
                        .await
                })
                .retry(&ExponentialBuilder::default().with_min_delay(Duration::new(10, 0)))
                .await?;

                println!("{} Signing Binary...", style("[6]").bold().dim());
                let binary = self
                    .sign_binary(&binary, api, global_options)
                    .await
                    .unwrap();

                Ok(binary)
            } else {
                Ok(binary)
            }
        } else if matches!(binary.state, BinaryState::Hashable) {
            println!("{} Start Binary Hashing...", style("[2]").bold().dim());
            // move to hashing
            let binary = self
                .change_binary_status(BinaryState::Hashing, binary, api, global_options)
                .await
                .unwrap();

            if self.signing_key_pair.is_some() || self.signing_key_private.is_some() {
                // wait for hashing to be signable
                println!("{} Waiting Binary to hash...", style("[3]").bold().dim());
                let binary = (|| async {
                    self.check_for_state_change(&binary, api, global_options)
                        .await
                })
                .retry(&ExponentialBuilder::default().with_min_delay(Duration::new(10, 0)))
                .await?;

                println!("{} Signing Binary...", style("[4]").bold().dim());
                let binary = self
                    .sign_binary(&binary, api, global_options)
                    .await
                    .unwrap();

                Ok(binary)
            } else {
                Ok(binary.clone())
            }
        } else if matches!(binary.state, BinaryState::Hashing) {
            if self.signing_key_pair.is_some() || self.signing_key_private.is_some() {
                // wait for hashing to be signable
                println!("{} Waiting Binary to hash...", style("[2]").bold().dim());
                let binary = (|| async {
                    self.check_for_state_change(binary, api, global_options)
                        .await
                })
                .retry(&ExponentialBuilder::default().with_min_delay(Duration::new(10, 0)))
                .await?;

                println!("{} Signing Binary...", style("[3]").bold().dim());
                let binary = self
                    .sign_binary(&binary, api, global_options)
                    .await
                    .unwrap();

                Ok(binary)
            } else {
                Ok(binary.clone())
            }
        } else if matches!(binary.state, BinaryState::Signable) {
            if self.signing_key_pair.is_some() || self.signing_key_private.is_some() {
                println!("{} Signing Binary...", style("[2]").bold().dim());
                let binary = self.sign_binary(binary, api, global_options).await.unwrap();

                Ok(binary)
            } else {
                Ok(binary.clone())
            }
        } else {
            Ok(binary.clone())
        }
    }

    async fn sign_binary(
        &self,
        binary: &Binary,
        api: &Api,
        global_options: &GlobalOptions,
    ) -> Result<Binary, Error> {
        let command = crate::api::binary_signatures::CreateCommand {
            binary_prn: binary.prn.clone(),
            binary_content_path: Some(self.content_path.clone().unwrap()),
            signature: None,
            signing_key_pair: self.signing_key_pair.clone(),
            signing_key_private: self.signing_key_private.clone(),
            signing_key_prn: self.signing_key_prn.clone(),
            api: Some(api.clone()),
        };

        match command.run(global_options.clone()).await? {
            Some(_) => {
                let binary = self
                    .change_binary_status(BinaryState::Signed, binary, api, global_options)
                    .await
                    .unwrap();

                Ok(binary)
            }
            None => panic!("Failed signing binary"),
        }
    }

    async fn check_for_state_change(
        &self,
        binary: &Binary,
        api: &Api,
        global_options: &GlobalOptions,
    ) -> Result<Binary, Error> {
        let command = GetCommand {
            prn: binary.prn.clone(),
            api: Some(api.to_owned()),
        };

        match command.run(global_options.clone()).await? {
            Some(GetBinaryResponse { binary }) => {
                if matches!(binary.state, BinaryState::Signable) {
                    Ok(binary)
                } else {
                    Err(Error::Api {
                        source: peridio_sdk::api::Error::Unknown {
                            error: "failed at checking binary state changes".to_string(),
                        },
                    })
                }
            }
            None => panic!("Cannot get binary to check for state change"),
        }
    }

    async fn change_binary_status(
        &self,
        state: BinaryState,
        binary: &Binary,
        api: &Api,
        global_options: &GlobalOptions,
    ) -> Result<Binary, Error> {
        let command = UpdateCommand {
            prn: binary.prn.clone(),
            description: None,
            state: Some(state),
            api: Some(api.clone()),
        };

        match command.run(global_options.clone()).await? {
            Some(UpdateBinaryResponse { binary }) => Ok(binary),
            None => panic!("Cannot update binary state"),
        }
    }

    async fn process_binary_parts(
        &self,
        binary: &Binary,
        api: &Api,
        global_options: &GlobalOptions,
    ) -> Result<(), Error> {
        println!("{} Calculating Binary Parts...", style("[2]").bold().dim());
        // get server parts
        let binary_parts = self
            .get_binary_parts(binary, api, global_options)
            .await
            .unwrap();

        let file_size = {
            let file = fs::File::open(self.content_path.clone().unwrap()).context(
                NonExistingPathSnafu {
                    path: &self.content_path.clone().unwrap(),
                },
            )?;

            file.metadata().unwrap().len()
        };

        let chucks_length =
            (file_size as f64 / self.binary_part_size.unwrap() as f64).ceil() as u64;

        let client = Client::new();

        println!("{} Uploading Binary Parts...", style("[3]").bold().dim());
        let pb = Arc::new(ProgressBar::new(file_size));
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));

        let result = stream::iter(1..=chucks_length)
            .map(|index| {
                let client = client.clone();
                let binary_part_size = self.binary_part_size.unwrap();
                let global_options = global_options.clone();
                let api = api.clone();
                let binary = binary.clone();
                let content_path = self.content_path.clone().unwrap();
                let binary_parts = binary_parts.to_vec();
                let pb = Arc::clone(&pb);
                tokio::spawn(async move {
                    // we ignore the ones we already created
                    if binary_parts.iter().any(|x| x.index as u64 == index) {
                        pb.inc(chucks_length);
                        return;
                    }

                    // we want to open the file in each thread, this is due to concurrency issues
                    // when using `Seek` from diferent threads theres a race condition in the data
                    let mut file = fs::File::open(&content_path).unwrap();

                    let file_position = binary_part_size * (index - 1);

                    file.seek(io::SeekFrom::Start(file_position)).unwrap();

                    let mut buffer = vec![0; binary_part_size.try_into().unwrap()];

                    let n = file.read(&mut buffer[..]).unwrap();

                    if n > 0 {
                        let mut mut_buffer = vec![0; n];

                        mut_buffer.copy_from_slice(&buffer[..n]);

                        let mut hasher = Sha256::new();
                        let _ = io::copy(&mut &mut_buffer[..], &mut hasher).unwrap();
                        let hash = hasher.finalize();

                        // push those bytes to the server
                        let create_command = crate::api::binary_parts::CreateCommand {
                            binary_prn: binary.prn.clone(),
                            expected_binary_size: Some(binary.size),
                            index: index as u16,
                            hash: format!("{hash:x}"),
                            api: Some(api),
                            size: n as u64,
                            binary_content_path: None,
                        };

                        let bin_part = create_command
                            .run(global_options)
                            .await
                            .expect("Error while creating a binary part binary part")
                            .expect("Cannot create a binary part");

                        // do amazon request
                        let body = Body::from(mut_buffer);

                        let hash_base64 = general_purpose::STANDARD.encode(hash);

                        let res = client
                            .put(bin_part.binary_part.presigned_upload_url)
                            .body(body)
                            .header("x-amz-checksum-sha256", &hash_base64)
                            .header("content-length", n)
                            .header("content-type", "application/octet-stream")
                            .send()
                            .await
                            .unwrap();

                        pb.inc(n.try_into().unwrap());

                        if !(200..=201).contains(&res.status().as_u16()) {
                            panic!("Wasn't able to upload binary to amazon S3")
                        };
                    };
                })
            })
            .buffer_unordered(self.concurrency.unwrap().into());

        let _ = result.collect::<Vec<_>>().await;

        pb.finish_and_clear();

        Ok(())
    }

    async fn get_binary_parts(
        &self,
        binary: &Binary,
        api: &Api,
        global_options: &GlobalOptions,
    ) -> Result<Vec<ListBinaryPart>, Error> {
        let list_command = crate::api::binary_parts::ListCommand {
            binary_prn: binary.prn.clone(),
            api: Some(api.clone()),
        };

        let binary_parts = match list_command.run(global_options.clone()).await? {
            Some(binary_parts) => binary_parts.binary_parts,
            None => Vec::new(),
        };

        Ok(binary_parts)
    }

    async fn get_or_create_binary(&self, api: &Api) -> Result<Option<CreateBinaryResponse>, Error> {
        println!("{} Creating Binary...", style("[1]").bold().dim());
        let organization_prn =
            Self::get_organization_prn_from_prn(self.artifact_version_prn.clone());

        let (size, hash) = if let Some(content_path) = &self.content_path {
            println!("{} Hashing Binary...", style("[1]").bold().dim());
            let mut file = fs::File::open(content_path).context(NonExistingPathSnafu {
                path: &content_path,
            })?;
            let mut hasher = Sha256::new();
            let _ = io::copy(&mut file, &mut hasher).unwrap();
            let hash = hasher.finalize();
            (file.metadata().unwrap().len(), format!("{hash:x}"))
        } else {
            (self.size.unwrap(), self.hash.clone().unwrap())
        };

        let list_params = ListBinariesParams {
            search: format!(
                "organization_prn:'{}' and target:'{}' and hash:'{}'",
                organization_prn, self.target, hash
            ),
            limit: None,
            order: None,
            page: None,
        };

        match api.binaries().list(list_params).await.context(ApiSnafu)? {
            Some(ListBinariesResponse {
                binaries,
                next_page: _,
            }) if binaries.len() == 1 => {
                // we found the binary, do as it was created
                println!("{} Resuming Binary...", style("[1]").bold().dim());
                let binary = binaries.first().unwrap().clone();
                Ok(Some(CreateBinaryResponse { binary }))
            }

            _ => {
                println!("{} Creating Binary...", style("[1]").bold().dim());
                // create the binary
                let params = CreateBinaryParams {
                    artifact_version_prn: self.artifact_version_prn.clone(),
                    description: self.description.clone(),
                    hash,
                    size,
                    target: self.target.clone(),
                };

                Ok(api.binaries().create(params).await.context(ApiSnafu)?)
            }
        }
    }

    fn get_organization_prn_from_prn(prn: String) -> String {
        // ["prn", "1", org_id, resource_name, resource_id]
        let prn: Vec<&str> = prn.split(':').collect();

        let org_id = prn[2];

        format!("prn:1:{org_id}")
    }
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self.inner.run(global_options).await? {
            Some(binary) => print_json!(&binary),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[arg(long)]
    pub limit: Option<u8>,
    #[arg(long)]
    pub order: Option<String>,
    #[arg(long)]
    pub search: String,
    #[arg(long)]
    pub page: Option<String>,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListBinariesParams {
            limit: self.inner.limit,
            order: self.inner.order,
            search: self.inner.search,
            page: self.inner.page,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.binaries().list(params).await.context(ApiSnafu)? {
            Some(binary) => print_json!(&binary),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    prn: String,

    #[clap(skip)]
    pub api: Option<Api>,
}

impl GetCommand {
    async fn run(self, global_options: GlobalOptions) -> Result<Option<GetBinaryResponse>, Error> {
        let params = GetBinaryParams { prn: self.prn };

        let api = if let Some(api) = self.api {
            api
        } else {
            Api::new(ApiOptions {
                api_key: global_options.api_key.unwrap(),
                endpoint: global_options.base_url,
                ca_bundle_path: global_options.ca_path,
            })
        };

        api.binaries().get(params).await.context(ApiSnafu)
    }
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self.inner.run(global_options).await? {
            Some(binary) => print_json!(&binary),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    #[arg(long)]
    prn: String,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long, value_parser = BinaryState::from_str)]
    pub state: Option<BinaryState>,

    #[clap(skip)]
    pub api: Option<Api>,
}

impl UpdateCommand {
    async fn run(
        self,
        global_options: GlobalOptions,
    ) -> Result<Option<UpdateBinaryResponse>, Error> {
        let params = UpdateBinaryParams {
            prn: self.prn,
            description: self.description,
            state: self.state,
        };

        let api = if let Some(api) = self.api {
            api
        } else {
            Api::new(ApiOptions {
                api_key: global_options.api_key.unwrap(),
                endpoint: global_options.base_url,
                ca_bundle_path: global_options.ca_path,
            })
        };

        api.binaries().update(params).await.context(ApiSnafu)
    }
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self.inner.run(global_options).await? {
            Some(device) => print_json!(&device),
            None => panic!(),
        }

        Ok(())
    }
}
