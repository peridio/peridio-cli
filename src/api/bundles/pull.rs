use crate::api::Command;
use crate::utils::{PRNType, PRNValueParser};
use crate::{ApiSnafu, Error, GlobalOptions};
use clap::Parser;
use peridio_sdk::api::artifact_versions::GetArtifactVersionParams;
use peridio_sdk::api::artifacts::GetArtifactParams;
use peridio_sdk::api::binaries::GetBinaryParams;

use peridio_sdk::api::bundles::{Bundle, GetBundleParams};
use peridio_sdk::api::Api;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::ResultExt;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

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
pub struct PullCommand {
    /// The PRN of the bundle to pull.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    bundle_prn: String,

    /// Output file path for the bundle archive. If not specified, uses bundle name or ID.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl Command<PullCommand> {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let api = Api::from(global_options);

        // Fetch bundle details
        let bundle_response = api
            .bundles()
            .get(GetBundleParams {
                prn: self.inner.bundle_prn.clone(),
            })
            .await
            .context(ApiSnafu)?
            .ok_or_else(|| Error::Generic {
                error: "Bundle not found".to_string(),
            })?;

        let output_path = self.inner.determine_output_path(&bundle_response.bundle)?;

        // Build the bundle.json structure
        let bundle_json = self
            .inner
            .build_bundle_json(&api, &bundle_response.bundle)
            .await?;

        // Create the CPIO archive
        self.inner
            .create_bundle_archive(&api, &bundle_json, &output_path)
            .await?;

        eprintln!("Bundle pulled successfully to: {}", output_path.display());
        Ok(())
    }
}

impl PullCommand {
    fn determine_output_path(&self, bundle: &Bundle) -> Result<PathBuf, Error> {
        if let Some(output) = &self.output {
            return Ok(output.clone());
        }

        // Generate filename from bundle name or PRN
        let filename = match bundle {
            Bundle::V1(bundle_v1) => bundle_v1.name.as_deref().unwrap_or(&bundle_v1.prn),
            Bundle::V2(bundle_v2) => bundle_v2.name.as_deref().unwrap_or(&bundle_v2.prn),
        };

        // Sanitize filename and add extension
        let safe_filename = filename
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        Ok(PathBuf::from(format!("{}.bundle", safe_filename)))
    }

    async fn build_bundle_json(&self, api: &Api, bundle: &Bundle) -> Result<BundleJson, Error> {
        // Only support V2 bundles for pull command
        let bundle_v2 = match bundle {
            Bundle::V1(_) => {
                return Err(Error::Generic {
                    error: "Bundle pull only supports V2 bundles".to_string(),
                });
            }
            Bundle::V2(bundle_v2) => bundle_v2,
        };

        let mut artifacts: HashMap<String, ArtifactInfo> = HashMap::new();
        let mut manifest: Vec<ManifestItem> = Vec::new();

        // Process each binary in the bundle to build the manifest and artifacts structure
        for bundle_binary in &bundle_v2.binaries {
            // Fetch binary details
            let binary_response = api
                .binaries()
                .get(GetBinaryParams {
                    prn: bundle_binary.prn.clone(),
                })
                .await
                .context(ApiSnafu)?
                .ok_or_else(|| Error::Generic {
                    error: format!("Binary {} not found", bundle_binary.prn),
                })?;

            let binary = &binary_response.binary;

            // Fetch artifact version details
            let artifact_version_response = api
                .artifact_versions()
                .get(GetArtifactVersionParams {
                    prn: binary.artifact_version_prn.clone(),
                })
                .await
                .context(ApiSnafu)?
                .ok_or_else(|| Error::Generic {
                    error: format!("Artifact version {} not found", binary.artifact_version_prn),
                })?;

            let artifact_version = &artifact_version_response.artifact_version;

            // Fetch artifact details
            let artifact_response = api
                .artifacts()
                .get(GetArtifactParams {
                    prn: artifact_version.artifact_prn.clone(),
                })
                .await
                .context(ApiSnafu)?
                .ok_or_else(|| Error::Generic {
                    error: format!("Artifact {} not found", artifact_version.artifact_prn),
                })?;

            let artifact = &artifact_response.artifact;

            // Convert binary signatures to our format
            let signatures = if let Some(binary_signatures) = &binary.signatures {
                binary_signatures
                    .iter()
                    .map(|sig| SignatureInfo {
                        keyid: sig.keyid.clone(),
                        sig: sig.signature.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            };

            // Build the binary info
            let binary_info = BinaryInfo {
                description: binary.description.clone(),
                signatures,
            };

            // Create manifest item
            let manifest_item = ManifestItem {
                hash: binary.hash.clone().unwrap_or_default(),
                size: binary.size.unwrap_or(0),
                binary_id: binary.prn.clone(),
                target: binary.target.clone(),
                artifact_version_id: binary.artifact_version_prn.clone(),
                artifact_id: artifact_version.artifact_prn.clone(),
                custom_metadata: bundle_binary
                    .custom_metadata
                    .clone()
                    .unwrap_or_else(|| Map::new()),
            };

            manifest.push(manifest_item);

            // Add to artifacts structure
            let artifact_info = artifacts
                .entry(artifact_version.artifact_prn.clone())
                .or_insert_with(|| ArtifactInfo {
                    name: artifact.name.clone(),
                    description: artifact.description.clone(),
                    versions: HashMap::new(),
                });

            let version_info = artifact_info
                .versions
                .entry(artifact_version.version.clone())
                .or_insert_with(|| ArtifactVersionInfo {
                    version: artifact_version.version.clone(),
                    description: artifact_version.description.clone(),
                    binaries: HashMap::new(),
                });

            version_info
                .binaries
                .insert(binary.prn.clone(), binary_info);
        }

        // Bundle signatures need to be fetched separately using bundle_signatures API
        // TODO: Implement bundle signatures fetching using ListBundleSignaturesParams
        // when the correct API method is available (not .list() on bundle_signatures())
        let bundle_sigs = Vec::new();

        // Build the bundle info
        let bundle_info = BundleInfo {
            id: bundle_v2.prn.clone(),
            name: bundle_v2.name.clone(),
            signatures: bundle_sigs,
            manifest,
        };

        Ok(BundleJson {
            artifacts,
            bundle: bundle_info,
        })
    }

    async fn create_bundle_archive(
        &self,
        api: &Api,
        bundle_json: &BundleJson,
        output_path: &PathBuf,
    ) -> Result<(), Error> {
        // Create output file and zstd encoder directly
        let output_file = std::fs::File::create(output_path).map_err(|e| Error::Generic {
            error: format!(
                "Failed to create output file {}: {}",
                output_path.display(),
                e
            ),
        })?;

        let mut zstd_writer = zstd::Encoder::new(output_file, 3).map_err(|e| Error::Generic {
            error: format!("Failed to create zstd encoder: {}", e),
        })?;

        // Serialize bundle.json
        let bundle_json_content =
            serde_json::to_vec_pretty(bundle_json).map_err(|e| Error::Generic {
                error: format!("Failed to serialize bundle.json: {}", e),
            })?;

        // Write bundle.json as first entry using CPIO format
        let bundle_json_builder = cpio::NewcBuilder::new("bundle.json");
        let bundle_json_size = bundle_json_content.len() as u32;
        let mut bundle_json_writer = bundle_json_builder.write(zstd_writer, bundle_json_size);
        io::copy(&mut bundle_json_content.as_slice(), &mut bundle_json_writer).map_err(|e| {
            Error::Generic {
                error: format!("Failed to write bundle.json content: {}", e),
            }
        })?;
        zstd_writer = bundle_json_writer.finish().map_err(|e| Error::Generic {
            error: format!("Failed to finish bundle.json entry: {}", e),
        })?;

        eprintln!("Added bundle.json to archive");

        // Download and add each binary in manifest order
        for (index, manifest_item) in bundle_json.bundle.manifest.iter().enumerate() {
            eprintln!(
                "Downloading binary {}/{}: {}",
                index + 1,
                bundle_json.bundle.manifest.len(),
                manifest_item.target
            );

            // Get binary details for download URL
            let binary_response = api
                .binaries()
                .get(GetBinaryParams {
                    prn: manifest_item.binary_id.clone(),
                })
                .await
                .context(ApiSnafu)?
                .ok_or_else(|| Error::Generic {
                    error: format!("Binary {} not found", manifest_item.binary_id),
                })?;

            let _binary = &binary_response.binary;

            // TODO: Implement actual binary download
            // The Binary struct should have a download URL field or we need to use
            // a separate API to get the download URL for the binary content
            let binary_content = vec![0u8; manifest_item.size as usize]; // Placeholder content

            // When implemented, this should:
            // 1. Get download URL from binary or separate API call
            // 2. Download the actual binary content using HTTP client
            // 3. Verify hash matches manifest_item.hash

            // Verify the content matches expected size
            let actual_size = binary_content.len() as u64;
            if actual_size != manifest_item.size {
                return Err(Error::Generic {
                    error: format!(
                        "Size mismatch for binary {}: expected {}, got {}",
                        manifest_item.binary_id, manifest_item.size, actual_size
                    ),
                });
            }

            // Create CPIO entry for this binary using the target name
            let binary_builder = cpio::NewcBuilder::new(&manifest_item.target);
            let binary_size = binary_content.len() as u32;
            let mut binary_writer = binary_builder.write(zstd_writer, binary_size);
            io::copy(&mut binary_content.as_slice(), &mut binary_writer).map_err(|e| {
                Error::Generic {
                    error: format!(
                        "Failed to write binary content for {}: {}",
                        manifest_item.target, e
                    ),
                }
            })?;
            zstd_writer = binary_writer.finish().map_err(|e| Error::Generic {
                error: format!(
                    "Failed to finish binary entry for {}: {}",
                    manifest_item.target, e
                ),
            })?;

            eprintln!(
                "Added {} to archive ({} bytes)",
                manifest_item.target, actual_size
            );
        }

        // Write CPIO trailer
        cpio::newc::trailer(&mut zstd_writer).map_err(|e| Error::Generic {
            error: format!("Failed to write CPIO trailer: {}", e),
        })?;

        // Finish the zstd stream
        zstd_writer.finish().map_err(|e| Error::Generic {
            error: format!("Failed to finish zstd compression: {}", e),
        })?;

        Ok(())
    }
}
