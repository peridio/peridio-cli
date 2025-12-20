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
    Binary, BinaryState, CreateBinaryParams, CreateBinaryResponse, GetBinaryParams,
    GetBinaryResponse,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BinaryInfo {
    pub description: Option<String>,
    pub signatures: Vec<SignatureInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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
    pub size: u64,
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

        // Only proceed with API calls if we have an API key
        if global_options.api_key.is_none() {
            return Err(Error::Generic {
                error: "API key is required for bundle push operation".to_string(),
            });
        }

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

        // Stream process the archive
        self.inner
            .stream_process_bundle(&api, &organization_prn)
            .await?;

        Ok(())
    }
}

impl PushCommand {
    async fn stream_process_bundle(&self, api: &Api, organization_prn: &str) -> Result<(), Error> {
        // First pass: extract and parse bundle.json
        let bundle_json = self.extract_bundle_json().await?;

        eprintln!(
            "Processing bundle with {} artifacts",
            bundle_json.artifacts.len()
        );

        // Create artifacts and versions first
        let binary_info_map = self
            .create_artifacts_and_versions(api, &bundle_json, organization_prn)
            .await?;

        // Second pass: stream process binaries
        let final_created_binaries = self
            .stream_process_binaries(api, &bundle_json, binary_info_map)
            .await?;

        // Create the bundle
        let bundle_prn = self
            .create_bundle(api, &bundle_json, &final_created_binaries, organization_prn)
            .await?;

        eprintln!("Bundle push completed successfully: {}", bundle_prn);
        Ok(())
    }

    async fn extract_bundle_json(&self) -> Result<BundleJson, Error> {
        // Open and decompress the zstd file
        let file = std::fs::File::open(&self.path).map_err(|e| Error::Generic {
            error: format!("Failed to open file {}: {}", self.path.display(), e),
        })?;

        let zstd_reader = zstd::Decoder::new(file).map_err(|e| Error::Generic {
            error: format!("Failed to create zstd decoder: {}", e),
        })?;

        let mut archive = zstd_reader;
        let mut bundle_json: Option<BundleJson> = None;

        // Stream through archive looking for bundle.json
        loop {
            let mut cpio_reader = cpio::NewcReader::new(archive).map_err(|e| Error::Generic {
                error: format!("Failed to create cpio reader: {}", e),
            })?;

            if cpio_reader.entry().is_trailer() {
                break;
            }

            let name = cpio_reader.entry().name().to_string();

            if name.ends_with("bundle.json") {
                let mut content = Vec::new();
                cpio_reader
                    .read_to_end(&mut content)
                    .map_err(|e| Error::Generic {
                        error: format!("Failed to read bundle.json content: {}", e),
                    })?;

                let json_str = String::from_utf8(content).map_err(|e| Error::Generic {
                    error: format!("Invalid UTF-8 in bundle.json: {}", e),
                })?;

                bundle_json =
                    Some(serde_json::from_str(&json_str).map_err(|e| Error::Generic {
                        error: format!("Failed to parse bundle.json: {}", e),
                    })?);

                eprintln!("Parsed bundle.json");
                break;
            }

            // Skip this entry and move to next
            archive = cpio_reader.finish().map_err(|e| Error::Generic {
                error: format!("Failed to finish reading cpio entry: {}", e),
            })?;
        }

        bundle_json.ok_or_else(|| Error::Generic {
            error: "bundle.json not found in archive".to_string(),
        })
    }

    async fn create_artifacts_and_versions(
        &self,
        api: &Api,
        bundle_json: &BundleJson,
        organization_prn: &str,
    ) -> Result<HashMap<String, (String, BinaryInfo)>, Error> {
        let mut binary_info_map: HashMap<String, (String, BinaryInfo)> = HashMap::new();

        // Create artifacts and versions (no binary content needed)
        for (artifact_id, artifact_info) in &bundle_json.artifacts {
            let artifact_prn = self
                .get_or_create_artifact(api, artifact_id, artifact_info, organization_prn)
                .await?;

            for (version_id, version_info) in &artifact_info.versions {
                let version_prn = self
                    .get_or_create_artifact_version(
                        api,
                        &artifact_prn,
                        version_id,
                        version_info,
                        organization_prn,
                    )
                    .await?;

                // Store binary info with version PRN for each binary
                for (binary_id, binary_info) in &version_info.binaries {
                    binary_info_map.insert(
                        binary_id.clone(),
                        (version_prn.clone(), binary_info.clone()),
                    );
                }
            }
        }

        Ok(binary_info_map)
    }

    async fn stream_process_binaries(
        &self,
        api: &Api,
        bundle_json: &BundleJson,
        binary_info_map: HashMap<String, (String, BinaryInfo)>,
    ) -> Result<HashMap<String, String>, Error> {
        let mut created_binaries: HashMap<String, String> = HashMap::new();

        // First pass: Check existing binary states to build ordered list of binaries to process
        let mut binaries_needing_processing = std::collections::HashSet::new();
        for manifest_item in &bundle_json.bundle.manifest {
            if let Some((version_prn, binary_info)) = binary_info_map.get(&manifest_item.binary_id)
            {
                let binary_prn =
                    self.construct_binary_prn(version_prn, &manifest_item.binary_id)?;
                let get_params = GetBinaryParams { prn: binary_prn };

                if let Ok(Some(GetBinaryResponse { binary })) =
                    api.binaries().get(get_params).await.context(ApiSnafu)
                {
                    // Check if binary is already in a final state
                    match binary.state {
                        BinaryState::Signable | BinaryState::Signed => {
                            eprintln!(
                                "Found existing binary with PRN {} in state {:?}. Skipping CPIO streaming.",
                                binary.prn, binary.state
                            );
                            created_binaries.insert(manifest_item.binary_id.clone(), binary.prn);
                            continue;
                        }
                        _ => {
                            // Need to process this binary
                            binaries_needing_processing.insert(manifest_item.binary_id.clone());
                        }
                    }
                } else {
                    // Binary doesn't exist - need to process
                    binaries_needing_processing.insert(manifest_item.binary_id.clone());
                }
            }
        }

        // If no binaries need processing, return early
        if binaries_needing_processing.is_empty() {
            eprintln!(
                "✓ All {} binaries already in desired state - skipped CPIO streaming entirely",
                created_binaries.len()
            );
            return Ok(created_binaries);
        }

        eprintln!(
            "Streaming {} of {} binaries from CPIO archive ({} already processed)",
            binaries_needing_processing.len(),
            bundle_json.bundle.manifest.len(),
            created_binaries.len()
        );

        // Second pass: Stream through archive using manifest order
        let file = std::fs::File::open(&self.path).map_err(|e| Error::Generic {
            error: format!("Failed to open file {}: {}", self.path.display(), e),
        })?;

        let zstd_reader = zstd::Decoder::new(file).map_err(|e| Error::Generic {
            error: format!("Failed to create zstd decoder: {}", e),
        })?;

        let mut archive = zstd_reader;
        let mut manifest_index = 0;

        // Stream through archive processing binaries in manifest order
        loop {
            let mut cpio_reader = cpio::NewcReader::new(archive).map_err(|e| Error::Generic {
                error: format!("Failed to create cpio reader: {}", e),
            })?;

            if cpio_reader.entry().is_trailer() {
                break;
            }

            let name = cpio_reader.entry().name().to_string();

            // Skip bundle.json and empty entries
            if name.ends_with("bundle.json") || name.is_empty() {
                archive = cpio_reader.finish().map_err(|e| Error::Generic {
                    error: format!("Failed to finish reading cpio entry: {}", e),
                })?;
                continue;
            }

            // Get the corresponding manifest item (assumes ordered archive)
            if manifest_index >= bundle_json.bundle.manifest.len() {
                return Err(Error::Generic {
                    error: format!(
                        "Archive contains more binaries than manifest entries. Expected {} binaries, found extra: {}",
                        bundle_json.bundle.manifest.len(), name
                    ),
                });
            }

            let manifest_item = &bundle_json.bundle.manifest[manifest_index];
            let binary_info_entry = binary_info_map.get(&manifest_item.binary_id);

            // Check if we need to process this binary
            if binaries_needing_processing.contains(&manifest_item.binary_id) {
                if let Some((version_prn, binary_info)) = binary_info_entry {
                    eprintln!("Processing binary file: {} (needs processing)", name);

                    // Read binary content into memory
                    let mut content = Vec::new();
                    cpio_reader
                        .read_to_end(&mut content)
                        .map_err(|e| Error::Generic {
                            error: format!("Failed to read binary content for {}: {}", name, e),
                        })?;

                    // Runtime verification: compute hash to verify ordering assumption
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(&content);
                    let computed_hash = format!("{:x}", hasher.finalize());

                    if computed_hash != manifest_item.hash {
                        return Err(Error::Generic {
                            error: format!(
                                "Archive ordering mismatch! Binary {} at position {} has hash {} but manifest expects {}. Archive binaries must match manifest order.",
                                name, manifest_index, computed_hash, manifest_item.hash
                            ),
                        });
                    }

                    // Process this binary immediately
                    let binary_prn = self
                        .create_and_upload_binary(
                            api,
                            version_prn,
                            &manifest_item.binary_id,
                            manifest_item,
                            binary_info,
                            &content,
                        )
                        .await?;

                    // Update the created_binaries map using binary_id
                    created_binaries.insert(manifest_item.binary_id.clone(), binary_prn);
                }
            } else {
                // Binary already processed - skip content reading entirely
                let content_size = cpio_reader.entry().file_size();
                std::io::copy(&mut cpio_reader.take(content_size), &mut std::io::sink()).map_err(
                    |e| Error::Generic {
                        error: format!("Failed to skip binary content for {}: {}", name, e),
                    },
                )?;
            }

            // Move to next entry
            archive = cpio_reader.finish().map_err(|e| Error::Generic {
                error: format!("Failed to finish reading cpio entry: {}", e),
            })?;
            manifest_index += 1;
        }

        // Verify we processed all expected binaries
        if manifest_index != bundle_json.bundle.manifest.len() {
            return Err(Error::Generic {
                error: format!(
                    "Archive contains fewer binaries than manifest entries. Expected {} binaries, found {}",
                    bundle_json.bundle.manifest.len(), manifest_index
                ),
            });
        }

        Ok(created_binaries)
    }

    async fn get_or_create_artifact(
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

        // Try to get existing artifact
        let get_result = api
            .artifacts()
            .get(get_params)
            .await
            .context(ApiSnafu)
            .map_err(|e| Error::Generic {
                error: format!(
                    "Failed to check for existing artifact {}: {}",
                    artifact_info.name, e
                ),
            })?;

        // If artifact exists, return its PRN
        if let Some(response) = get_result {
            eprintln!("Found existing artifact: {}", artifact_info.name);
            return Ok(response.artifact.prn);
        }

        // Artifact doesn't exist, create it
        let create_params = CreateArtifactParams {
            custom_metadata: None,
            description: artifact_info.description.clone(),
            id: Some(artifact_id.to_string()),
            name: artifact_info.name.clone(),
        };

        let create_result = api
            .artifacts()
            .create(create_params)
            .await
            .context(ApiSnafu)?;

        match create_result {
            Some(response) => {
                eprintln!("Created new artifact: {}", artifact_info.name);
                Ok(response.artifact.prn)
            }
            None => Err(Error::Generic {
                error: "No response from artifact creation".to_string(),
            }),
        }
    }

    async fn get_or_create_artifact_version(
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

        // Try to get existing artifact version
        let get_result = api
            .artifact_versions()
            .get(get_params)
            .await
            .context(ApiSnafu)
            .map_err(|e| Error::Generic {
                error: format!(
                    "Failed to check for existing artifact version v{}: {}",
                    version_info.version, e
                ),
            })?;

        // If artifact version exists, return its PRN
        if let Some(response) = get_result {
            eprintln!("Found existing artifact version: v{}", version_info.version);
            return Ok(response.artifact_version.prn);
        }

        // Artifact version doesn't exist, create it
        let create_params = CreateArtifactVersionParams {
            artifact_prn: artifact_prn.to_string(),
            custom_metadata: None,
            description: version_info.description.clone(),
            id: Some(version_id.to_string()),
            version: version_info.version.clone(),
        };

        let create_result = api
            .artifact_versions()
            .create(create_params)
            .await
            .context(ApiSnafu)?;

        match create_result {
            Some(response) => {
                eprintln!("Created new artifact version: v{}", version_info.version);
                Ok(response.artifact_version.prn)
            }
            None => Err(Error::Generic {
                error: "No response from artifact version creation".to_string(),
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
        binary_content: &[u8],
    ) -> Result<String, Error> {
        // Hash verification was already done in stream_process_binaries, so skip it here

        let binary_prn = self.construct_binary_prn(version_prn, binary_id)?;

        // Try to get existing binary directly
        let get_params = GetBinaryParams { prn: binary_prn };

        let get_result = api.binaries().get(get_params).await.context(ApiSnafu)?;

        // If binary doesn't exist, create it
        let Some(GetBinaryResponse { binary }) = get_result else {
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
        };

        // Binary exists - validate hash
        if !self.is_binary_hash_valid(&binary, manifest_item, binary_id) {
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

        // Binary exists and hash is valid - handle based on state
        self.handle_existing_binary(&binary, binary_info, manifest_item, binary_content, api)
            .await
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
            size: manifest_item.size,
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

    fn construct_binary_prn(&self, version_prn: &str, binary_id: &str) -> Result<String, Error> {
        let prn_builder = PRNBuilder::from_prn(version_prn).map_err(|e| Error::Generic {
            error: format!("Failed to parse version PRN {}: {}", version_prn, e),
        })?;

        prn_builder.binary(binary_id).map_err(|e| Error::Generic {
            error: format!("Failed to construct binary PRN: {}", e),
        })
    }

    fn is_binary_hash_valid(
        &self,
        binary: &Binary,
        manifest_item: &ManifestItem,
        binary_id: &str,
    ) -> bool {
        match &binary.hash {
            Some(existing_hash) => {
                if existing_hash != &manifest_item.hash {
                    eprintln!(
                        "Found binary with binary_id {} but hash mismatch. Expected: {}, Found: {}. Creating new binary.",
                        binary_id, manifest_item.hash, existing_hash
                    );
                    false
                } else {
                    true
                }
            }
            None => {
                eprintln!(
                    "Found binary with binary_id {} but it has no hash. This may be an incomplete binary. Creating new binary.",
                    binary_id
                );
                false
            }
        }
    }

    async fn handle_existing_binary(
        &self,
        binary: &Binary,
        binary_info: &BinaryInfo,
        manifest_item: &ManifestItem,
        binary_content: &[u8],
        api: &Api,
    ) -> Result<String, Error> {
        match binary.state {
            BinaryState::Uploadable | BinaryState::Hashable | BinaryState::Hashing => {
                eprintln!(
                    "Found existing binary with PRN {} in state {:?}. Processing through state transitions.",
                    binary.prn,
                    binary.state
                );

                self.process_binary_with_signatures(
                    binary,
                    binary_info,
                    manifest_item,
                    binary_content,
                    api,
                )
                .await
            }
            BinaryState::Signable | BinaryState::Signed => {
                eprintln!(
                    "Found existing binary with PRN {} in state {:?}. Using existing binary instead of creating new one.",
                    binary.prn,
                    binary.state
                );
                Ok(binary.prn.clone())
            }
            _ => {
                eprintln!(
                    "Found binary with PRN {} but it's in state {:?} which is not processable. Creating new binary.",
                    binary.prn,
                    binary.state
                );
                Err(Error::Generic {
                    error: format!(
                        "Binary in unsupported state {:?} for processing",
                        binary.state
                    ),
                })
            }
        }
    }

    async fn process_binary_with_signatures(
        &self,
        binary: &Binary,
        binary_info: &BinaryInfo,
        manifest_item: &ManifestItem,
        binary_content: &[u8],
        api: &Api,
    ) -> Result<String, Error> {
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
            .process_binary(binary, api, Some(binary_content))
            .await?;

        Ok(processed_binary.prn)
    }
}
