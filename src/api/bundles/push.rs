use super::super::binary_processor::{BinaryProcessor, BinaryProcessorConfig, SignatureConfig};

use super::super::Command;

use crate::utils::PRNBuilder;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::artifact_versions::{CreateArtifactVersionParams, GetArtifactVersionParams};
use peridio_sdk::api::artifacts::{CreateArtifactParams, GetArtifactParams};
use peridio_sdk::api::binaries::{
    BinaryState, CreateBinaryParams, CreateBinaryResponse, GetBinaryParams, GetBinaryResponse,
};
use peridio_sdk::api::bundles::{
    Bundle, CreateBundleBinary, CreateBundleParams, CreateBundleParamsV2,
};
use peridio_sdk::api::Api;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::Digest;
use snafu::ResultExt;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

// Bundle JSON structure definitions
#[derive(Debug, Deserialize, Serialize)]
pub struct BundleJson {
    pub artifacts: HashMap<String, ArtifactInfo>,
    pub bundle: BundleInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ArtifactInfo {
    pub name: String,
    pub description: Option<String>,
    pub versions: HashMap<String, ArtifactVersionInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ArtifactVersionInfo {
    pub version: String,
    pub description: Option<String>,
    pub binaries: HashMap<String, BinaryInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BinaryInfo {
    pub description: Option<String>,
    pub signatures: Vec<SignatureInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignatureInfo {
    pub keyid: String,
    pub sig: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BundleInfo {
    pub id: String,
    pub name: Option<String>,
    pub signatures: Vec<SignatureInfo>,
    pub manifest: Vec<ManifestItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ManifestItem {
    pub hash: String,
    pub length: u64,
    pub binary_id: String,
    pub target: String,
    pub artifact_version_id: String,
    pub artifact_id: String,
    pub custom_metadata: Map<String, Value>,
}

#[derive(Parser, Debug)]
pub struct PushCommand {
    /// The path to the zstd compressed cpio archive containing the bundle and binaries
    #[arg(short = 'p', long)]
    path: PathBuf,

    /// The size to use when creating binary parts. All binary parts will be equal to this size, except the last one which will be less than or equal to this size.
    #[arg(
        long,
        default_value = "5242880",
        value_parser = clap::value_parser!(u64).range(5242880..50000000000),
    )]
    binary_part_size: Option<u64>,

    /// Limit the concurrency of jobs that create and upload binary parts. [default: 2x the core count, to a maximum of 16]
    #[arg(long)]
    concurrency: Option<u8>,

    /// The name of a signing key pair in your Peridio CLI config. This will dictate both the private key to create a binary signature with as well as the signing key Peridio will use to verify the binary signature.
    #[arg(
        long,
        short = 's',
        conflicts_with = "signing_key_private",
        conflicts_with = "signing_key_prn"
    )]
    signing_key_pair: Option<String>,

    /// The private key to create a binary signature with.
    #[arg(
        long,
        conflicts_with = "signing_key_pair",
        requires = "signing_key_prn"
    )]
    signing_key_private: Option<String>,

    /// The PRN of the signing key Peridio will use to verify the binary signature.
    #[arg(
        long,
        conflicts_with = "signing_key_pair",
        requires = "signing_key_private",
        value_parser = crate::utils::PRNValueParser::new(crate::utils::PRNType::SigningKey)
    )]
    signing_key_prn: Option<String>,

    #[clap(skip)]
    global_options: Option<GlobalOptions>,
}

impl Command<PushCommand> {
    pub async fn run(mut self, global_options: GlobalOptions) -> Result<(), Error> {
        // Store global options in the command for use in binary upload
        self.inner.global_options = Some(global_options.clone());
        eprintln!("Opening archive: {}", self.inner.path.display());

        // Open and decompress the zstd file
        let file = std::fs::File::open(&self.inner.path).map_err(|e| Error::Generic {
            error: format!("Failed to open file {}: {}", self.inner.path.display(), e),
        })?;

        let zstd_reader = zstd::Decoder::new(file).map_err(|e| Error::Generic {
            error: format!("Failed to create zstd decoder: {}", e),
        })?;

        // Extract the cpio archive
        let mut bundle_json: Option<BundleJson> = None;
        let mut binary_files: HashMap<String, Vec<u8>> = HashMap::new();
        let mut archive = zstd_reader;

        // Read all entries from the cpio archive using a loop
        loop {
            let mut cpio_reader = cpio::NewcReader::new(archive).map_err(|e| Error::Generic {
                error: format!("Failed to create cpio reader: {}", e),
            })?;

            // Check if this is the trailer (end of archive)
            if cpio_reader.entry().is_trailer() {
                break;
            }

            let name = cpio_reader.entry().name().to_string();
            let mut content = Vec::new();

            // Read the file content
            cpio_reader
                .read_to_end(&mut content)
                .map_err(|e| Error::Generic {
                    error: format!("Failed to read file content for {}: {}", name, e),
                })?;

            if name.ends_with("bundle.json") {
                // Parse the bundle.json
                let json_str = String::from_utf8(content).map_err(|e| Error::Generic {
                    error: format!("Invalid UTF-8 in bundle.json: {}", e),
                })?;

                bundle_json =
                    Some(serde_json::from_str(&json_str).map_err(|e| Error::Generic {
                        error: format!("Failed to parse bundle.json: {}", e),
                    })?);

                eprintln!("Parsed bundle.json");
            } else if !name.is_empty() && !content.is_empty() && !name.ends_with("bundle.json") {
                // Store binary files for later upload
                eprintln!("Extracted binary file: {}", name);
                binary_files.insert(name, content);
            }

            // Move to next entry
            archive = cpio_reader.finish().map_err(|e| Error::Generic {
                error: format!("Failed to finish reading cpio entry: {}", e),
            })?;
        }

        let bundle_json = bundle_json.ok_or_else(|| Error::Generic {
            error: "bundle.json not found in archive".to_string(),
        })?;

        eprintln!(
            "Processing bundle with {} artifacts",
            bundle_json.artifacts.len()
        );

        // Only proceed with API calls if we have an API key
        if global_options.api_key.is_some() {
            let api = Api::from(global_options);

            // Get organization PRN from current user
            let organization_prn = match api.users().me().await.context(ApiSnafu)? {
                Some(user_response) => user_response.data.organization_prn,
                None => {
                    return Err(Error::Generic {
                        error: "Unable to get current user information".to_string(),
                    });
                }
            };

            // Process resources in order: artifact -> artifact versions -> binaries -> bundle
            self.inner
                .process_bundle(&api, bundle_json, binary_files, &organization_prn)
                .await?;
        } else {
            return Err(Error::Generic {
                error: "API key is required for bundle push operation".to_string(),
            });
        }

        Ok(())
    }
}

impl PushCommand {
    async fn process_bundle(
        &self,
        api: &Api,
        bundle_json: BundleJson,
        binary_files: HashMap<String, Vec<u8>>,
        organization_prn: &str,
    ) -> Result<(), Error> {
        // Step 1: Get or create artifacts (log errors and continue)
        let mut created_artifacts = HashMap::new();
        for (artifact_id, artifact_info) in &bundle_json.artifacts {
            match self
                .create_or_get_artifact(api, artifact_id, artifact_info, organization_prn)
                .await
            {
                Ok(prn) => {
                    created_artifacts.insert(artifact_id.clone(), prn);
                }
                Err(e) => {
                    eprintln!(
                        "⚠ Failed to get/create artifact {}: {}",
                        artifact_info.name, e
                    );
                    // Continue processing other artifacts
                }
            }
        }

        // Step 2: Get or create artifact versions (log errors and continue)
        let mut created_versions = HashMap::new();
        for (artifact_id, artifact_info) in &bundle_json.artifacts {
            if let Some(artifact_prn) = created_artifacts.get(artifact_id) {
                for (version_id, version_info) in &artifact_info.versions {
                    match self
                        .create_or_get_artifact_version(
                            api,
                            artifact_prn,
                            version_id,
                            version_info,
                            organization_prn,
                        )
                        .await
                    {
                        Ok(prn) => {
                            created_versions.insert(version_id.clone(), prn);
                        }
                        Err(e) => {
                            eprintln!(
                                "⚠ Failed to get/create artifact version {} v{}: {}",
                                artifact_info.name, version_info.version, e
                            );
                            // Continue processing other versions
                        }
                    }
                }
            }
        }

        // Step 3: Create binaries (fail on error) and upload content
        let mut created_binaries = HashMap::new();
        for manifest_item in &bundle_json.bundle.manifest {
            let artifact_info = bundle_json
                .artifacts
                .get(&manifest_item.artifact_id)
                .ok_or_else(|| Error::Generic {
                    error: format!(
                        "Artifact {} not found in bundle.json",
                        manifest_item.artifact_id
                    ),
                })?;

            let version_info = artifact_info
                .versions
                .get(&manifest_item.artifact_version_id)
                .ok_or_else(|| Error::Generic {
                    error: format!(
                        "Artifact version {} not found",
                        manifest_item.artifact_version_id
                    ),
                })?;

            let binary_info = version_info
                .binaries
                .get(&manifest_item.binary_id)
                .ok_or_else(|| Error::Generic {
                    error: format!(
                        "Binary {} not found in artifact version {}. Available binaries: {}",
                        manifest_item.binary_id,
                        manifest_item.artifact_version_id,
                        version_info
                            .binaries
                            .keys()
                            .map(|k| k.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                })?;

            if let Some(version_prn) = created_versions.get(&manifest_item.artifact_version_id) {
                match self
                    .create_and_upload_binary(
                        api,
                        version_prn,
                        &manifest_item.binary_id,
                        manifest_item,
                        binary_info,
                        &binary_files,
                    )
                    .await
                {
                    Ok(prn) => {
                        created_binaries.insert(manifest_item.binary_id.clone(), prn);
                        eprintln!("✓ Binary processed: {}", manifest_item.binary_id);
                    }
                    Err(e) => {
                        return Err(Error::Generic {
                            error: format!(
                                "Failed to create/upload binary {}: {}",
                                manifest_item.binary_id, e
                            ),
                        });
                    }
                }
            }
        }

        // Step 4: Create bundle (fail on error)
        match self
            .create_bundle(api, &bundle_json, &created_binaries, organization_prn)
            .await
        {
            Ok(_) => {
                eprintln!("✓ Bundle processed successfully");
                Ok(())
            }
            Err(e) => Err(Error::Generic {
                error: format!("Failed to create bundle: {}", e),
            }),
        }
    }

    async fn create_or_get_artifact(
        &self,
        api: &Api,
        artifact_id: &str,
        artifact_info: &ArtifactInfo,
        organization_prn: &str,
    ) -> Result<String, Error> {
        // Try to get existing artifact first using PRN utilities
        let prn_builder = PRNBuilder::from_prn(organization_prn).map_err(|e| Error::Generic {
            error: format!(
                "Failed to parse organization PRN {}: {}",
                organization_prn, e
            ),
        })?;

        let artifact_prn = prn_builder
            .artifact(artifact_id)
            .map_err(|e| Error::Generic {
                error: format!("Failed to construct artifact PRN: {}", e),
            })?;

        let get_params = GetArtifactParams { prn: artifact_prn };

        match api.artifacts().get(get_params).await.context(ApiSnafu) {
            Ok(Some(response)) => {
                // Artifact already exists, return its PRN
                eprintln!("Found existing artifact: {}", artifact_info.name);
                Ok(response.artifact.prn)
            }
            Ok(None) => {
                // Artifact doesn't exist, create it
                let create_params = CreateArtifactParams {
                    custom_metadata: None,
                    description: artifact_info.description.clone(),
                    id: Some(artifact_id.to_string()),
                    name: artifact_info.name.clone(),
                };

                match api
                    .artifacts()
                    .create(create_params)
                    .await
                    .context(ApiSnafu)
                {
                    Ok(Some(response)) => {
                        eprintln!("Created new artifact: {}", artifact_info.name);
                        Ok(response.artifact.prn)
                    }
                    Ok(None) => Err(Error::Generic {
                        error: "No response from artifact creation".to_string(),
                    }),
                    Err(e) => Err(Error::Generic {
                        error: format!("Failed to create artifact {}: {}", artifact_info.name, e),
                    }),
                }
            }
            Err(e) => Err(Error::Generic {
                error: format!(
                    "Failed to check for existing artifact {}: {}",
                    artifact_info.name, e
                ),
            }),
        }
    }

    async fn create_or_get_artifact_version(
        &self,
        api: &Api,
        artifact_prn: &str,
        version_id: &str,
        version_info: &ArtifactVersionInfo,
        organization_prn: &str,
    ) -> Result<String, Error> {
        // Try to get existing artifact version first
        // Use PRN utilities to construct artifact version PRN
        let prn_builder = PRNBuilder::from_prn(organization_prn).map_err(|e| Error::Generic {
            error: format!(
                "Failed to parse organization PRN {}: {}",
                organization_prn, e
            ),
        })?;

        let artifact_version_prn =
            prn_builder
                .artifact_version(version_id)
                .map_err(|e| Error::Generic {
                    error: format!("Failed to construct artifact version PRN: {}", e),
                })?;

        let get_params = GetArtifactVersionParams {
            prn: artifact_version_prn,
        };

        match api
            .artifact_versions()
            .get(get_params)
            .await
            .context(ApiSnafu)
        {
            Ok(Some(response)) => {
                // Artifact version already exists, return its PRN
                eprintln!("Found existing artifact version: v{}", version_info.version);
                Ok(response.artifact_version.prn)
            }
            Ok(None) => {
                // Artifact version doesn't exist, create it
                let create_params = CreateArtifactVersionParams {
                    artifact_prn: artifact_prn.to_string(),
                    custom_metadata: None,
                    description: version_info.description.clone(),
                    id: Some(version_id.to_string()),
                    version: version_info.version.clone(),
                };

                match api
                    .artifact_versions()
                    .create(create_params)
                    .await
                    .context(ApiSnafu)
                {
                    Ok(Some(response)) => {
                        eprintln!("Created new artifact version: v{}", version_info.version);
                        Ok(response.artifact_version.prn)
                    }
                    Ok(None) => Err(Error::Generic {
                        error: "No response from artifact version creation".to_string(),
                    }),
                    Err(e) => Err(Error::Generic {
                        error: format!(
                            "Failed to create artifact version v{}: {}",
                            version_info.version, e
                        ),
                    }),
                }
            }
            Err(e) => Err(Error::Generic {
                error: format!(
                    "Failed to check for existing artifact version v{}: {}",
                    version_info.version, e
                ),
            }),
        }
    }

    async fn create_and_upload_binary(
        &self,
        api: &Api,
        version_prn: &str,
        binary_id: &str,
        manifest_item: &ManifestItem,
        binary_info: &BinaryInfo,
        binary_files: &HashMap<String, Vec<u8>>,
    ) -> Result<String, Error> {
        // Find the binary content by matching the hash or by binary_id filename
        let binary_content = binary_files
            .values()
            .find(|content| {
                let mut hasher = sha2::Sha256::new();
                hasher.update(content);
                let hash = format!("{:x}", hasher.finalize());
                hash == manifest_item.hash
            })
            .or_else(|| {
                // Fallback: try to find by binary_id as filename
                binary_files.get(binary_id)
            })
            .or_else(|| {
                // Another fallback: try to find the first non-bundle.json file
                binary_files
                    .iter()
                    .find(|(name, _)| !name.ends_with("bundle.json"))
                    .map(|(_, content)| content)
            })
            .ok_or_else(|| Error::Generic {
                error: format!(
                    "Binary content not found for binary_id {} with hash {}. Available files: {}",
                    binary_id,
                    manifest_item.hash,
                    binary_files
                        .keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })?;

        // Use PRN utilities to construct binary PRN from version PRN
        let prn_builder = PRNBuilder::from_prn(version_prn).map_err(|e| Error::Generic {
            error: format!("Failed to parse version PRN {}: {}", version_prn, e),
        })?;

        let binary_prn = prn_builder.binary(binary_id).map_err(|e| Error::Generic {
            error: format!("Failed to construct binary PRN: {}", e),
        })?;

        // Try to get existing binary directly
        let get_params = GetBinaryParams { prn: binary_prn };

        match api.binaries().get(get_params).await.context(ApiSnafu)? {
            Some(GetBinaryResponse { binary }) => {
                // Validate that the existing binary has the expected hash
                match &binary.hash {
                    Some(existing_hash) => {
                        if existing_hash != &manifest_item.hash {
                            eprintln!(
                                "Found binary with id {} but hash mismatch. Expected: {}, Found: {}. Creating new binary.",
                                binary_id, manifest_item.hash, existing_hash
                            );
                            return self
                                .create_new_binary(
                                    api,
                                    version_prn,
                                    binary_id,
                                    manifest_item,
                                    binary_info,
                                    binary_content,
                                )
                                .await;
                        }
                    }
                    None => {
                        eprintln!(
                            "Found binary with id {} but it has no hash. This may be an incomplete binary. Creating new binary.",
                            binary_id
                        );
                        return self
                            .create_new_binary(
                                api,
                                version_prn,
                                binary_id,
                                manifest_item,
                                binary_info,
                                binary_content,
                            )
                            .await;
                    }
                }

                // Check binary state and handle accordingly
                match binary.state {
                    BinaryState::Signed => {
                        eprintln!("Found existing signed binary with id: {}", binary_id);
                        Ok(binary.prn)
                    }
                    BinaryState::Signable => {
                        eprintln!(
                            "Found existing binary with id {} in state {:?}. Using existing binary instead of creating new one.",
                            binary_id, binary.state
                        );
                        Ok(binary.prn)
                    }
                    BinaryState::Uploadable | BinaryState::Hashable | BinaryState::Hashing => {
                        eprintln!(
                            "Found existing binary with id {} in state {:?}. Processing through state transitions.",
                            binary_id, binary.state
                        );

                        // Convert bundle signatures to BinaryProcessor format
                        let signatures: Vec<SignatureConfig> = binary_info
                            .signatures
                            .iter()
                            .map(|sig_info| {
                                SignatureConfig::pre_computed(
                                    sig_info.keyid.clone(),
                                    sig_info.sig.clone(),
                                )
                            })
                            .collect();

                        let config = BinaryProcessorConfig {
                            binary_part_size: self.binary_part_size,
                            concurrency: self.concurrency,
                            global_options: self.global_options.clone().unwrap(),
                            binary_content_hash: Some(manifest_item.hash.clone()),
                            content_path: None, // Bundles work with in-memory content
                            signatures,
                        };

                        let processor = BinaryProcessor::new(config);
                        let processed_binary = processor
                            .process_binary(&binary, api, Some(binary_content))
                            .await?;

                        Ok(processed_binary.prn)
                    }
                    _ => {
                        eprintln!(
                            "Found binary with id {} but it's in state {:?} which is not processable. Creating new binary.",
                            binary_id, binary.state
                        );
                        self.create_new_binary(
                            api,
                            version_prn,
                            binary_id,
                            manifest_item,
                            binary_info,
                            binary_content,
                        )
                        .await
                    }
                }
            }
            None => {
                // No binary exists with this id, create it
                eprintln!("No binary found with id {}, creating new binary", binary_id);
                self.create_new_binary(
                    api,
                    version_prn,
                    binary_id,
                    manifest_item,
                    binary_info,
                    binary_content,
                )
                .await
            }
        }
    }

    async fn create_new_binary(
        &self,
        api: &Api,
        version_prn: &str,
        binary_id: &str,
        manifest_item: &ManifestItem,
        binary_info: &BinaryInfo,
        binary_content: &[u8],
    ) -> Result<String, Error> {
        let params = CreateBinaryParams {
            artifact_version_prn: version_prn.to_string(),
            custom_metadata: if manifest_item.custom_metadata.is_empty() {
                None
            } else {
                Some(manifest_item.custom_metadata.clone())
            },
            description: binary_info.description.clone(),
            hash: manifest_item.hash.clone(),
            id: Some(binary_id.to_string()),
            size: manifest_item.length,
            target: manifest_item.target.clone(),
        };

        match api.binaries().create(params).await.context(ApiSnafu) {
            Ok(Some(CreateBinaryResponse { binary })) => {
                // Convert bundle signatures to BinaryProcessor format
                let signatures: Vec<SignatureConfig> = binary_info
                    .signatures
                    .iter()
                    .map(|sig_info| {
                        SignatureConfig::pre_computed(sig_info.keyid.clone(), sig_info.sig.clone())
                    })
                    .collect();

                let config = BinaryProcessorConfig {
                    binary_part_size: self.binary_part_size,
                    concurrency: self.concurrency,
                    global_options: self.global_options.clone().unwrap(),
                    binary_content_hash: Some(manifest_item.hash.clone()),
                    content_path: None, // Bundles work with in-memory content
                    signatures,
                };

                let processor = BinaryProcessor::new(config);
                let processed_binary = processor
                    .process_binary(&binary, api, Some(binary_content))
                    .await?;

                eprintln!(
                    "Created and processed new binary: {} (state: {:?})",
                    binary_id, processed_binary.state
                );
                Ok(processed_binary.prn)
            }
            Ok(None) => Err(Error::Generic {
                error: "No response from binary creation".to_string(),
            }),
            Err(e) => Err(Error::Generic {
                error: format!("Failed to create binary: {}", e),
            }),
        }
    }

    async fn create_bundle(
        &self,
        api: &Api,
        bundle_json: &BundleJson,
        created_binaries: &HashMap<String, String>,
        _organization_prn: &str,
    ) -> Result<String, Error> {
        // Always create a new bundle
        let binaries: Vec<CreateBundleBinary> = bundle_json
            .bundle
            .manifest
            .iter()
            .filter_map(|manifest_item| {
                created_binaries
                    .get(&manifest_item.binary_id)
                    .map(|binary_prn| CreateBundleBinary {
                        prn: binary_prn.clone(),
                        custom_metadata: if manifest_item.custom_metadata.is_empty() {
                            None
                        } else {
                            Some(manifest_item.custom_metadata.clone())
                        },
                    })
            })
            .collect();

        let create_params = CreateBundleParams::V2(CreateBundleParamsV2 {
            binaries,
            id: Some(bundle_json.bundle.id.clone()),
            name: bundle_json.bundle.name.clone(),
        });

        match api.bundles().create(create_params).await.context(ApiSnafu) {
            Ok(Some(response)) => {
                let prn = match &response.bundle {
                    Bundle::V1(bundle) => &bundle.prn,
                    Bundle::V2(bundle) => &bundle.prn,
                };
                eprintln!("Bundle: {}", bundle_json.bundle.id);
                Ok(prn.to_string())
            }
            Ok(None) => Err(Error::Generic {
                error: "No response from bundle creation".to_string(),
            }),
            Err(e) => Err(Error::Generic {
                error: format!("Failed to create bundle: {}", e),
            }),
        }
    }
}
