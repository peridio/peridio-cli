use super::Command;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::utils::maybe_json;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use crate::NonExistingPathSnafu;
use backon::ConstantBuilder;
use backon::Retryable;
use base64::engine::general_purpose;
use base64::Engine;
use clap::Parser;
use futures_util::stream;
use futures_util::StreamExt;
use indicatif::ProgressBar;
use indicatif::ProgressState;
use indicatif::ProgressStyle;
use peridio_sdk::api::artifact_versions::ArtifactVersion;
use peridio_sdk::api::artifact_versions::GetArtifactVersionParams;
use peridio_sdk::api::artifact_versions::GetArtifactVersionResponse;
use peridio_sdk::api::artifacts::Artifact;
use peridio_sdk::api::artifacts::GetArtifactParams;
use peridio_sdk::api::artifacts::GetArtifactResponse;
use peridio_sdk::api::binaries::Binary;
use peridio_sdk::api::binaries::BinaryState;
use peridio_sdk::api::binaries::CreateBinaryParams;
use peridio_sdk::api::binaries::CreateBinaryResponse;
use peridio_sdk::api::binaries::DeleteBinaryParams;
use peridio_sdk::api::binaries::GetBinaryParams;
use peridio_sdk::api::binaries::GetBinaryResponse;
use peridio_sdk::api::binaries::ListBinariesParams;
use peridio_sdk::api::binaries::ListBinariesResponse;
use peridio_sdk::api::binaries::UpdateBinaryParams;
use peridio_sdk::api::binaries::UpdateBinaryResponse;
use peridio_sdk::api::binary_parts::BinaryPartState;
use peridio_sdk::api::binary_parts::ListBinaryPart;
use peridio_sdk::api::bundle_overrides::AddDeviceParams;
use peridio_sdk::api::bundle_overrides::BundleOverride;
use peridio_sdk::api::bundle_overrides::CreateBundleOverrideParams;
use peridio_sdk::api::bundles::Bundle;
use peridio_sdk::api::releases::CreateReleaseParams;
use peridio_sdk::api::releases::Release;
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
use reqwest::Body;
use reqwest::Client;
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use std::cmp;
use std::io::Read;
use std::io::Seek;
use std::sync::Arc;
use std::thread::available_parallelism;
use std::time::Duration;
use std::{fs, io};
use time::OffsetDateTime;

#[derive(Debug, serde::Serialize)]
pub struct CreateBinaryCommandResponse {
    pub binary: Binary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle: Option<Bundle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_override: Option<BundleOverride>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<Release>,
}

#[derive(Parser, Debug)]
pub enum BinariesCommand {
    Create(Box<Command<CreateCommand>>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
    Delete(Command<DeleteCommand>),
}

impl BinariesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
        }
    }
}

/// Idempotently create, upload in parallel, and sign a binary.
///
/// This command idempotently: creates a binary record, uploads its content in parallel via binary parts, and creates a binary signature.
#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The PRN of the artifact version you wish to create a binary for.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::ArtifactVersion)
    )]
    artifact_version_prn: String,

    /// A JSON object that informs the metadata that will be associated with this binary when it is included in bundles.
    #[arg(long, conflicts_with = "custom_metadata_path")]
    custom_metadata: Option<String>,

    /// The path to the JSON file value for custom_metadata
    #[arg(long, conflicts_with = "custom_metadata")]
    custom_metadata_path: Option<String>,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,

    /// The lowercase hex encoding of the SHA256 hash of the binary's content.
    #[arg(
        long,
        conflicts_with = "content_path",
        required_unless_present = "content_path"
    )]
    hash: Option<String>,

    /// A user provided custom UUID id for the binary database record.
    #[arg(long)]
    id: Option<String>,

    /// The expected size in bytes of the binary.
    #[arg(
        long,
        conflicts_with = "content_path",
        required_unless_present = "content_path"
    )]
    size: Option<u64>,

    /// An arbitrary string attached to the resource. Often a target triplet to indicate compatibility.
    #[arg(long)]
    target: String,

    /// The path to the file you wish to upload as the binary's content.
    #[arg(
        long,
        conflicts_with_all = ["hash", "size"],
        required_unless_present_any = ["hash", "size"],
    )]
    content_path: Option<String>,

    /// The size to use when creating binary parts. All binary parts will be equal to this size, except the last one which will be less than or equal to this size.
    #[arg(
        long,
        requires = "content_path",
        default_value = "5242880",
        value_parser = clap::value_parser!(u64).range(5242880..50000000000),
    )]
    binary_part_size: Option<u64>,

    /// Limit the concurrency of jobs that create and upload binary parts. [default: 2x the core count, to a maximum of 16]
    #[arg(long, requires = "content_path")]
    concurrency: Option<u8>,

    /// The name of a signing key pair in your Peridio CLI config. This will dictate both the private key to create a binary signature with as well as the signing key Peridio will use to verify the binary signature.
    #[arg(
        long,
        short = 's',
        conflicts_with = "signing_key_private",
        conflicts_with = "signing_key_prn",
        required_unless_present_any = ["signing_key_private", "signing_key_prn", "skip_upload"],
    )]
    signing_key_pair: Option<String>,

    /// A path to a PKCS#8 private key encoded as a pem to create a binary signature binary with.
    #[arg(
        long,
        conflicts_with = "signing_key_pair",
        required_unless_present_any = ["signing_key_pair", "skip_upload"],
        requires = "signing_key_prn"
    )]
    signing_key_private: Option<String>,

    /// The PRN of the signing key Peridio will use to verify the binary signature.
    #[arg(
        long,
        conflicts_with = "signing_key_pair",
        required_unless_present_any = ["signing_key_pair", "skip_upload"],
        requires = "signing_key_private",
        value_parser = PRNValueParser::new(PRNType::SigningKey)
    )]
    signing_key_prn: Option<String>,

    /// Create the binary record but do not upload its content nor sign it.
    #[arg(
        long,
        default_value = "false",
        conflicts_with = "concurrency",
        conflicts_with = "binary_part_size",
        conflicts_with = "signing_key_pair"
    )]
    skip_upload: bool,

    /// The PRN of the bundle override to associate with this binary.
    ///
    /// A bundle will be created for the newly-created binary.
    ///
    /// The given bundle override will be updated to this bundle.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::BundleOverride),
        conflicts_with_all = ["device_prn", "cohort_prn"]
    )]
    bundle_override_prn: Option<String>,

    /// The PRN of the device to stage this binary for.
    ///
    /// A bundle will be created for the newly-created binary.
    ///
    /// A bundle override will be created with this bundle.
    ///
    /// The given device will be added to the bundle override.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Device),
        conflicts_with_all = ["bundle_override_prn", "cohort_prn"]
    )]
    device_prn: Option<String>,

    /// The PRN of a cohort, in which to create a release for.
    ///
    /// A bundle will be created for the newly-created binary.
    ///
    /// A release will be created for the given cohort with this bundle.
    ///
    /// The created release is not required, has scheduled availability of "now", and 100% availability.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Cohort),
        conflicts_with_all = ["bundle_override_prn", "device_prn"]
    )]
    cohort_prn: Option<String>,

    #[clap(skip)]
    global_options: Option<GlobalOptions>,
}

impl CreateCommand {
    async fn run(
        &mut self,
        global_options: GlobalOptions,
    ) -> Result<Option<CreateBinaryCommandResponse>, Error> {
        let api = Api::from(global_options.clone());
        self.global_options = Some(global_options.clone());

        let binary = match self.get_or_create_binary(&api).await? {
            Some(CreateBinaryResponse { binary }) => {
                if self.skip_upload {
                    return Ok(Some(CreateBinaryCommandResponse {
                        binary,
                        bundle: None,
                        bundle_override: None,
                        release: None,
                    }));
                }

                if self.concurrency.is_none() {
                    // default to 2x the core count
                    self.concurrency = Some(
                        cmp::min(available_parallelism().unwrap().get() * 2, 16)
                            .try_into()
                            .unwrap(),
                    );
                }

                let binary = self.process_binary(&binary, &api).await?;

                let bundle = if self.bundle_override_prn.is_some()
                    || self.device_prn.is_some()
                    || self.cohort_prn.is_some()
                {
                    Some(self.create_bundle_for_binary(&binary, &api).await?)
                } else {
                    None
                };

                let mut bundle_override = None;
                let mut release = None;

                if let Some(bundle_override_prn) = &self.bundle_override_prn {
                    bundle_override = Some(
                        self.handle_create_with_bundle_override(
                            bundle_override_prn,
                            bundle.as_ref().unwrap(),
                            &api,
                        )
                        .await?,
                    );
                }

                if let Some(device_prn) = &self.device_prn {
                    bundle_override = Some(
                        self.handle_create_with_device(device_prn, bundle.as_ref().unwrap(), &api)
                            .await?,
                    );
                }

                if let Some(cohort_prn) = &self.cohort_prn {
                    release = Some(
                        self.handle_create_with_cohort(cohort_prn, bundle.as_ref().unwrap(), &api)
                            .await?,
                    );
                }

                Some(CreateBinaryCommandResponse {
                    binary,
                    bundle,
                    bundle_override,
                    release,
                })
            }
            None => None,
        };

        Ok(binary)
    }

    async fn handle_create_with_bundle_override(
        &self,
        bundle_override_prn: &str,
        bundle: &Bundle,
        api: &Api,
    ) -> Result<BundleOverride, Error> {
        let bundle_override = self
            .update_bundle_override_to_bundle(bundle_override_prn, &bundle.prn, api)
            .await?;

        Ok(bundle_override)
    }

    async fn handle_create_with_device(
        &self,
        device_prn: &str,
        bundle: &Bundle,
        api: &Api,
    ) -> Result<BundleOverride, Error> {
        let bundle_override = self.create_bundle_override_for_bundle(bundle, api).await?;

        self.add_device_to_bundle_override(device_prn, &bundle_override, api)
            .await?;

        Ok(bundle_override)
    }

    async fn handle_create_with_cohort(
        &self,
        cohort_prn: &str,
        bundle: &Bundle,
        api: &Api,
    ) -> Result<Release, Error> {
        let release = self
            .create_release_for_bundle(bundle, cohort_prn, api)
            .await?;

        Ok(release)
    }

    async fn add_device_to_bundle_override(
        &self,
        device_prn: &str,
        bundle_override: &BundleOverride,
        api: &Api,
    ) -> Result<(), Error> {
        eprintln!("Adding device to bundle override...");

        let add_device_params = AddDeviceParams {
            prn: bundle_override.prn.clone(),
            device_prn: device_prn.to_string(),
        };

        match api
            .bundle_overrides()
            .add_device(add_device_params)
            .await
            .context(ApiSnafu)?
        {
            Some(_) => {
                eprintln!(
                    "Added device {} to bundle override {}",
                    device_prn, bundle_override.prn
                );
                Ok(())
            }
            None => Err(Error::Generic {
                error: format!(
                    "Failed to add device {} to bundle override {}",
                    device_prn, bundle_override.prn
                ),
            }),
        }
    }

    async fn create_bundle_for_binary(&self, binary: &Binary, api: &Api) -> Result<Bundle, Error> {
        let (artifact_version, artifact) = self
            .get_artifact_and_version(&binary.artifact_version_prn, api)
            .await?;

        let bundle = self
            .create_bundle(&artifact, &artifact_version, binary, api)
            .await?;

        Ok(bundle)
    }

    async fn get_artifact_and_version(
        &self,
        artifact_version_prn: &str,
        api: &Api,
    ) -> Result<(ArtifactVersion, Artifact), Error> {
        let artifact_version_params = GetArtifactVersionParams {
            prn: artifact_version_prn.to_string(),
        };

        let artifact_version = match api
            .artifact_versions()
            .get(artifact_version_params)
            .await
            .context(ApiSnafu)?
        {
            Some(GetArtifactVersionResponse { artifact_version }) => artifact_version,
            None => {
                return Err(Error::Generic {
                    error: format!("Failed to get artifact version: {artifact_version_prn}"),
                });
            }
        };

        let artifact_params = GetArtifactParams {
            prn: artifact_version.artifact_prn.clone(),
        };

        let artifact = match api
            .artifacts()
            .get(artifact_params)
            .await
            .context(ApiSnafu)?
        {
            Some(GetArtifactResponse { artifact }) => artifact,
            None => {
                return Err(Error::Generic {
                    error: format!("Failed to get artifact: {}", artifact_version.artifact_prn),
                });
            }
        };

        Ok((artifact_version, artifact))
    }

    async fn create_bundle(
        &self,
        artifact: &Artifact,
        artifact_version: &ArtifactVersion,
        binary: &Binary,
        api: &Api,
    ) -> Result<Bundle, Error> {
        let name = format!(
            "{}@{}/{}",
            artifact.name, artifact_version.version, binary.target
        );

        eprintln!("Fetching or creating bundle for binary...");

        let bundle_params = peridio_sdk::api::bundles::CreateBundleParams {
            artifact_version_prns: vec![artifact_version.prn.clone()],
            id: None,
            name: Some(name),
        };

        match api
            .bundles()
            .create(bundle_params)
            .await
            .context(ApiSnafu)?
        {
            Some(bundle_response) => {
                eprintln!("Using bundle {}", bundle_response.bundle.prn);
                Ok(bundle_response.bundle)
            }
            None => Err(Error::Generic {
                error: "Failed to get bundle for binary".to_string(),
            }),
        }
    }

    async fn update_bundle_override_to_bundle(
        &self,
        bundle_override_prn: &str,
        bundle_prn: &str,
        api: &Api,
    ) -> Result<BundleOverride, Error> {
        eprintln!("Updating bundle override...",);

        let update_params = peridio_sdk::api::bundle_overrides::UpdateBundleOverrideParams {
            prn: bundle_override_prn.to_string(),
            bundle_prn: Some(bundle_prn.to_string()),
            name: None,
            description: None,
            ends_at: None,
            starts_at: None,
        };

        match api
            .bundle_overrides()
            .update(update_params)
            .await
            .context(ApiSnafu)?
        {
            Some(response) => {
                eprintln!("Updated bundle override {bundle_override_prn} to bundle {bundle_prn}");
                Ok(response.bundle_override)
            }
            None => Err(Error::Generic {
                error: "Failed to update bundle override".to_string(),
            }),
        }
    }

    async fn create_bundle_override_for_bundle(
        &self,
        bundle: &Bundle,
        api: &Api,
    ) -> Result<BundleOverride, Error> {
        eprintln!("Creating bundle override...");

        // Use the bundle name if available, otherwise use a default name
        let name = bundle
            .name
            .clone()
            .unwrap_or_else(|| format!("Bundle Override for {}", bundle.prn));

        // Use current time as start time (ISO 8601 format)
        let starts_at = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| Error::Generic {
                error: format!("Failed to format current time: {e}"),
            })?;

        let create_params = CreateBundleOverrideParams {
            name,
            bundle_prn: bundle.prn.clone(),
            starts_at,
            description: Some(format!(
                "Auto-created bundle override for bundle {}",
                bundle.prn
            )),
            ends_at: None, // No end time, active indefinitely
        };

        match api
            .bundle_overrides()
            .create(create_params)
            .await
            .context(ApiSnafu)?
        {
            Some(response) => {
                eprintln!("Created bundle override {}", response.bundle_override.prn);
                Ok(response.bundle_override)
            }
            None => Err(Error::Generic {
                error: "Failed to create bundle override".to_string(),
            }),
        }
    }

    async fn create_release_for_bundle(
        &self,
        bundle: &Bundle,
        cohort_prn: &str,
        api: &Api,
    ) -> Result<Release, Error> {
        eprintln!(
            "Creating release for bundle {} in cohort {}",
            bundle.prn, cohort_prn
        );

        let name = bundle
            .name
            .clone()
            .unwrap_or_else(|| format!("Release for {}", bundle.prn));

        let schedule_date = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| Error::Generic {
                error: format!("Failed to format current time: {e}"),
            })?;

        let create_params = CreateReleaseParams {
            bundle_prn: bundle.prn.clone(),
            cohort_prn: cohort_prn.to_string(),
            name,
            schedule_date,
            phase_value: Some(1.0),
            required: false,
            description: None,
            disabled: None,
            next_release_prn: None,
            phase_mode: None,
            phase_tags: None,
            previous_release_prn: None,
            version: None,
            version_requirement: None,
        };

        match api
            .releases()
            .create(create_params)
            .await
            .context(ApiSnafu)?
        {
            Some(response) => {
                eprintln!("Created release {}", response.release.prn);
                Ok(response.release)
            }
            None => Err(Error::Generic {
                error: "Failed to create release".to_string(),
            }),
        }
    }

    async fn process_binary(&self, binary: &Binary, api: &Api) -> Result<Binary, Error> {
        if matches!(binary.state, BinaryState::Uploadable) {
            let binary = self.process_binary_parts(binary, api).await?;

            // do signing if available
            if self.signing_key_pair.is_some() || self.signing_key_private.is_some() {
                // wait for hashing to be signable
                eprintln!("Waiting for cloud hashing...");
                let binary = (|| async { self.check_for_state_change(&binary, api).await })
                    .retry(
                        &ConstantBuilder::default()
                            .with_delay(Duration::new(10, 0))
                            .with_max_times(30),
                    )
                    .await?;

                eprintln!("Signing binary...");
                let binary = self.sign_binary(&binary, api).await?;

                Ok(binary)
            } else {
                Ok(binary)
            }
        } else if matches!(binary.state, BinaryState::Hashable) {
            eprintln!("Updating binary to hashing...");
            // move to hashing
            let binary = self
                .change_binary_status(ArgBinaryState::Hashing, binary, api)
                .await?;

            if self.signing_key_pair.is_some() || self.signing_key_private.is_some() {
                // wait for hashing to be signable
                eprintln!("Waiting for cloud hashing...");
                let binary = (|| async { self.check_for_state_change(&binary, api).await })
                    .retry(
                        &ConstantBuilder::default()
                            .with_delay(Duration::new(10, 0))
                            .with_max_times(30),
                    )
                    .await?;

                eprintln!("Signing binary...");
                let binary = self.sign_binary(&binary, api).await?;

                Ok(binary)
            } else {
                Ok(binary.clone())
            }
        } else if matches!(binary.state, BinaryState::Hashing) {
            if self.signing_key_pair.is_some() || self.signing_key_private.is_some() {
                // wait for hashing to be signable
                eprintln!("Waiting for cloud hashing...");
                let binary = (|| async { self.check_for_state_change(binary, api).await })
                    .retry(
                        &ConstantBuilder::default()
                            .with_delay(Duration::new(10, 0))
                            .with_max_times(30),
                    )
                    .await?;

                eprintln!("Signing binary...");
                let binary = self.sign_binary(&binary, api).await?;

                Ok(binary)
            } else {
                Ok(binary.clone())
            }
        } else if matches!(binary.state, BinaryState::Signable) {
            if self.signing_key_pair.is_some() || self.signing_key_private.is_some() {
                eprintln!("Signing binary...");
                let binary = self.sign_binary(binary, api).await?;

                Ok(binary)
            } else {
                Ok(binary.clone())
            }
        } else {
            Ok(binary.clone())
        }
    }

    async fn sign_binary(&self, binary: &Binary, api: &Api) -> Result<Binary, Error> {
        let command = crate::api::binary_signatures::CreateCommand {
            binary_prn: binary.prn.clone(),
            binary_content_path: self.content_path.clone(),
            signature: None,
            signing_key_pair: self.signing_key_pair.clone(),
            signing_key_private: self.signing_key_private.clone(),
            signing_key_prn: self.signing_key_prn.clone(),
            api: Some(api.clone()),
            binary_content_hash: binary.hash.clone(),
        };

        match command
            .run(self.global_options.clone().ok_or_else(|| Error::Api {
                source: peridio_sdk::api::Error::Unknown {
                    error: "Global options not available".to_string(),
                },
            })?)
            .await?
        {
            Some(_) => {
                let binary = self
                    .change_binary_status(ArgBinaryState::Signed, binary, api)
                    .await?;

                Ok(binary)
            }
            None => panic!("Failed signing binary"),
        }
    }

    async fn check_for_state_change(&self, binary: &Binary, api: &Api) -> Result<Binary, Error> {
        let command = GetCommand {
            prn: binary.prn.clone(),
            api: Some(api.to_owned()),
        };

        match command
            .run(self.global_options.clone().ok_or_else(|| Error::Api {
                source: peridio_sdk::api::Error::Unknown {
                    error: "Global options not available".to_string(),
                },
            })?)
            .await?
        {
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
        state: ArgBinaryState,
        binary: &Binary,
        api: &Api,
    ) -> Result<Binary, Error> {
        let command = UpdateCommand {
            prn: binary.prn.clone(),
            custom_metadata: None,
            description: None,
            state: Some(state),
            api: Some(api.clone()),
            hash: None,
            size: None,
        };

        match command
            .run(self.global_options.clone().ok_or_else(|| Error::Api {
                source: peridio_sdk::api::Error::Unknown {
                    error: "Global options not available".to_string(),
                },
            })?)
            .await?
        {
            Some(UpdateBinaryResponse { binary }) => Ok(binary),
            None => panic!("Cannot update binary state"),
        }
    }

    async fn process_binary_parts(&self, binary: &Binary, api: &Api) -> Result<Binary, Error> {
        eprintln!("Evaluating binary parts...");
        // get server parts
        let binary_parts = self.get_binary_parts(binary, api).await?;

        let file_size = {
            let content_path = self.content_path.clone().ok_or_else(|| Error::Api {
                source: peridio_sdk::api::Error::Unknown {
                    error: "Content path is required for binary upload".to_string(),
                },
            })?;
            let file = fs::File::open(&content_path).context(NonExistingPathSnafu {
                path: &content_path,
            })?;

            file.metadata()
                .map_err(|e| Error::Api {
                    source: peridio_sdk::api::Error::Unknown {
                        error: format!("Failed to get file metadata for '{content_path}': {e}"),
                    },
                })?
                .len()
        };

        let binary_part_size = self.binary_part_size.ok_or_else(|| Error::Api {
            source: peridio_sdk::api::Error::Unknown {
                error: "Binary part size is required".to_string(),
            },
        })?;
        let chunks_length = (file_size as f64 / binary_part_size as f64).ceil() as u64;

        let client = Client::new();

        self.upload_binary_parts(
            binary,
            api,
            file_size,
            chunks_length,
            &client,
            &binary_parts,
        )
        .await?;

        eprintln!("Validating Upload");
        // list binary parts again in order to get the latest state
        let binary_parts = self.get_binary_parts(binary, api).await?;

        // if the parts are not equal it means we missed a part
        // if a binary part state is not valid is because something is missing
        if !(binary_parts.len() == chunks_length as usize
            && binary_parts
                .iter()
                .all(|x| matches!(x.state, BinaryPartState::Valid)))
        {
            // retry only once
            eprintln!("Retrying Upload");
            self.upload_binary_parts(
                binary,
                api,
                file_size,
                chunks_length,
                &client,
                &binary_parts,
            )
            .await?;
        }

        eprintln!("Updating binary to hashable...");
        // we created the binary parts not move it to hashable
        let binary = self
            .change_binary_status(ArgBinaryState::Hashable, binary, api)
            .await?;

        eprintln!("Updating binary to hashing...",);
        // move to hashing
        let binary = self
            .change_binary_status(ArgBinaryState::Hashing, &binary, api)
            .await?;

        Ok(binary)
    }

    #[allow(clippy::too_many_arguments)]
    async fn upload_binary_parts(
        &self,
        binary: &Binary,
        api: &Api,
        file_size: u64,
        chunks_length: u64,
        client: &Client,
        binary_parts: &[ListBinaryPart],
    ) -> Result<(), Error> {
        eprintln!("Creating binary parts and uploading...");
        let pb = Arc::new(ProgressBar::new(file_size));
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));

        let result = stream::iter(1..=chunks_length)
            .map(|index| {
                let client = client.clone();
                let binary_part_size = self.binary_part_size.unwrap();
                let global_options = self.global_options.clone().unwrap();
                let api = api.clone();
                let binary = binary.clone();
                let content_path = self.content_path.clone().unwrap();
                let binary_parts = binary_parts.to_vec();
                let pb = Arc::clone(&pb);
                tokio::spawn(async move {
                    // we ignore the ones we already created
                    if let Some(binary_part) = binary_parts.iter().find(|x| x.index as u64 == index)
                    {
                        if matches!(binary_part.state, BinaryPartState::Valid) {
                            pb.inc(binary_part.size);
                            return;
                        }
                    }

                    // we want to open the file in each thread, this is due to concurrency issues
                    // when using `Seek` from different threads theres a race condition in the data
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
                            expected_binary_size: binary.size,
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
    ) -> Result<Vec<ListBinaryPart>, Error> {
        let list_command = crate::api::binary_parts::ListCommand {
            binary_prn: binary.prn.clone(),
            api: Some(api.clone()),
        };

        let binary_parts = match list_command
            .run(self.global_options.clone().unwrap())
            .await?
        {
            Some(binary_parts) => binary_parts.binary_parts,
            None => Vec::new(),
        };

        Ok(binary_parts)
    }

    async fn get_or_create_binary(&self, api: &Api) -> Result<Option<CreateBinaryResponse>, Error> {
        let (size, hash) = if let Some(content_path) = &self.content_path {
            eprintln!("Hashing binary...");
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
            list: ListParams {
                search: Some(format!(
                    "target:'{}' and artifact_version_prn:'{}'",
                    self.target, self.artifact_version_prn
                )),
                limit: None,
                order: None,
                page: None,
            },
        };

        match api.binaries().list(list_params).await.context(ApiSnafu)? {
            Some(ListBinariesResponse {
                binaries,
                next_page: _,
            }) if binaries.len() == 1 => {
                eprintln!("Binary already exists...");
                let binary = binaries.first().unwrap().clone();

                // Check if hash of local binary matches hash of remote binary.
                let binary = if binary.hash != Some(hash.clone()) || binary.size != Some(size) {
                    // Hash of local binary does not match hash of remote binary.

                    // Check if the binary is signed.
                    if matches!(binary.state, BinaryState::Signed) {
                        // Because the hashes did not match and the binary is signed, we fail with
                        // an error since signed binaries cannot be overriden.
                        let artifact_version_params = GetArtifactVersionParams {
                            prn: binary.artifact_version_prn.clone(),
                        };
                        let fallback_message = format!(
                  "A signed binary already exists for \"{}\" with a target of \"{}\". Once a binary is signed, it cannot be overwritten.",
                  binary.artifact_version_prn, binary.target
              );
                        let message = match api
                            .artifact_versions()
                            .get(artifact_version_params)
                            .await
                        {
                            Ok(Some(GetArtifactVersionResponse { artifact_version })) => {
                                let artifact_params = GetArtifactParams {
                                    prn: artifact_version.artifact_prn.clone(),
                                };
                                match api.artifacts().get(artifact_params).await {
                                    Ok(Some(GetArtifactResponse { artifact })) => {
                                        format!(
                                  "A signed binary already exists for artifact \"{}\" at version \"{}\" with target \"{}\". Once a binary is signed, it cannot be overwritten.",
                                  artifact.name,
                                  artifact_version.version,
                                  binary.target
                              )
                                    }
                                    _ => fallback_message,
                                }
                            }
                            _ => fallback_message,
                        };
                        return Err(Error::Generic { error: message });
                    } else {
                        // Because the hashes did not match and the binary is not signed, we can
                        // reset it.
                        self.reset_binary(&binary, hash, size, api).await?
                    }
                } else {
                    binary
                };

                Ok(Some(CreateBinaryResponse { binary }))
            }

            _ => {
                eprintln!("Creating binary...");
                let custom_metadata =
                    if let Some(custom_metadata_path) = self.custom_metadata_path.clone() {
                        fs::read_to_string(&custom_metadata_path)
                            .context(NonExistingPathSnafu {
                                path: &custom_metadata_path,
                            })?
                            .into()
                    } else {
                        self.custom_metadata.clone()
                    };

                // create the binary
                let params = CreateBinaryParams {
                    artifact_version_prn: self.artifact_version_prn.clone(),
                    custom_metadata: maybe_json(custom_metadata),
                    description: self.description.clone(),
                    hash,
                    id: self.id.clone(),
                    size,
                    target: self.target.clone(),
                };

                Ok(api.binaries().create(params).await.context(ApiSnafu)?)
            }
        }
    }

    async fn reset_binary(
        &self,
        binary: &Binary,
        hash: String,
        size: u64,
        api: &Api,
    ) -> Result<Binary, Error> {
        let binary = self
            .change_binary_status(ArgBinaryState::Uploadable, binary, api)
            .await?;

        let update_command = UpdateCommand {
            prn: binary.prn.clone(),
            custom_metadata: None,
            description: None,
            hash: Some(hash),
            size: Some(size),
            state: None,
            api: Some(api.clone()),
        };

        // update hash and size
        let binary = match update_command
            .run(self.global_options.clone().unwrap())
            .await?
        {
            Some(UpdateBinaryResponse { binary }) => binary,
            None => panic!(),
        };

        Ok(binary)
    }
}

impl Command<CreateCommand> {
    async fn run(mut self, global_options: GlobalOptions) -> Result<(), Error> {
        match self.inner.run(global_options).await? {
            Some(response) => print_json!(&response),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The PRN of the resource to delete.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Binary)
    )]
    prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteBinaryParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        api.binaries().delete(params).await.context(ApiSnafu)?;

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[clap(flatten)]
    list_args: ListArgs,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListBinariesParams {
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

        match api.binaries().list(params).await.context(ApiSnafu)? {
            Some(binary) => print_json!(&binary),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    /// The PRN of the resource to get.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Binary)
    )]
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
            Api::from(global_options)
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
    /// The PRN of the resource you wish to update.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Binary)
    )]
    prn: String,

    /// A JSON object that informs the metadata that will be associated with this binary when it is included in bundles.
    #[arg(long)]
    pub custom_metadata: Option<String>,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    pub description: Option<String>,

    /// The state to transition the binary to.
    #[arg(long, value_enum)]
    pub state: Option<ArgBinaryState>,

    /// The lowercase hex encoding of the SHA256 hash of the binary's content.
    #[arg(long)]
    pub hash: Option<String>,

    /// The size of the binary in bytes.
    #[arg(long)]
    pub size: Option<u64>,

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
            custom_metadata: maybe_json(self.custom_metadata),
            description: self.description,
            state: self.state.map(BinaryState::from),
            hash: self.hash,
            size: self.size,
        };

        let api = if let Some(api) = self.api {
            api
        } else {
            Api::from(global_options)
        };

        api.binaries().update(params).await.context(ApiSnafu)
    }
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self.inner.run(global_options).await? {
            Some(binary) => print_json!(&binary),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ArgBinaryState {
    Destroyed,
    Hashable,
    Hashing,
    Signable,
    Signed,
    Uploadable,
}

impl From<ArgBinaryState> for BinaryState {
    fn from(other: ArgBinaryState) -> BinaryState {
        match other {
            ArgBinaryState::Destroyed => BinaryState::Destroyed,
            ArgBinaryState::Hashable => BinaryState::Hashable,
            ArgBinaryState::Hashing => BinaryState::Hashing,
            ArgBinaryState::Signable => BinaryState::Signable,
            ArgBinaryState::Signed => BinaryState::Signed,
            ArgBinaryState::Uploadable => BinaryState::Uploadable,
        }
    }
}
