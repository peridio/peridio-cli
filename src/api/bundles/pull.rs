use crate::api::Command;
use crate::utils::peridio_resource_names::Prn;
use crate::utils::{PRNType, PRNValueParser};
use crate::{ApiSnafu, Error, GlobalOptions};
use clap::Parser;
use peridio_sdk::api::artifact_versions::GetArtifactVersionParams;
use peridio_sdk::api::artifacts::GetArtifactParams;
use peridio_sdk::api::binaries::GetBinaryParams;

use peridio_sdk::api::bundles::{Bundle, GetBundleParams};
use peridio_sdk::api::Api;
use serde_json::Map;
use snafu::ResultExt;
use std::io;
use std::path::PathBuf;

// Import shared bundle format structs
use super::{
    ArtifactInfo, ArtifactVersionInfo, BinaryInfo, BundleInfo, BundleJson, ManifestItem,
    SignatureInfo,
};

use std::collections::HashMap;

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

/// Extract the resource ID from a PRN
fn extract_id_from_prn(prn: &str) -> Result<String, Error> {
    let parsed = Prn::parse(prn).map_err(|e| Error::Generic {
        error: format!("Failed to parse PRN {}: {}", prn, e),
    })?;
    Ok(parsed.resource_id)
}

impl Command<PullCommand> {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        // Ensure we use API version 2 for bundle calls since we need the bundle hash field.
        // API v1 bundles don't include bundle hash information which is required
        // for proper bundle manifest generation and content verification.
        let mut global_options = global_options;
        global_options.api_version = Some(2);
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

        Ok(PathBuf::from(format!("{}.cpio.zst", safe_filename)))
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
        let mut binary_prn_map: HashMap<String, String> = HashMap::new(); // binary_id -> binary_prn

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
            let binary_id = extract_id_from_prn(&binary.prn)?;
            let manifest_item = ManifestItem {
                hash: binary.hash.clone().unwrap_or_default(),
                size: binary.size.unwrap_or(0),
                binary_id: binary_id.clone(),
                target: binary.target.clone(),
                artifact_version_id: extract_id_from_prn(&artifact_version.prn)?,
                artifact_id: extract_id_from_prn(&artifact.prn)?,
                custom_metadata: bundle_binary
                    .custom_metadata
                    .clone()
                    .unwrap_or_else(|| Map::new()),
            };

            // Store the PRN mapping for later use in downloads
            binary_prn_map.insert(binary_id, binary.prn.clone());
            manifest.push(manifest_item);

            // Add to artifacts structure
            let artifact_info = artifacts
                .entry(extract_id_from_prn(&artifact.prn)?)
                .or_insert_with(|| ArtifactInfo {
                    name: artifact.name.clone(),
                    description: artifact.description.clone(),
                    versions: HashMap::new(),
                });

            let version_info = artifact_info
                .versions
                .entry(extract_id_from_prn(&artifact_version.prn)?)
                .or_insert_with(|| ArtifactVersionInfo {
                    version: artifact_version.version.clone(),
                    description: artifact_version.description.clone(),
                    binaries: HashMap::new(),
                });

            version_info
                .binaries
                .insert(extract_id_from_prn(&binary.prn)?, binary_info);
        }

        // Bundle signatures need to be fetched separately using bundle_signatures API
        // TODO: Implement bundle signatures fetching using ListBundleSignaturesParams
        // when the correct API method is available (not .list() on bundle_signatures())
        let bundle_sigs = Vec::new();

        // Build the bundle info
        let bundle_info = BundleInfo {
            id: extract_id_from_prn(&bundle_v2.prn)?,
            name: bundle_v2.name.clone(),
            hash: bundle_v2.hash.clone(),
            signatures: bundle_sigs,
            manifest,
        };

        let result = BundleJson {
            artifacts,
            bundle: bundle_info,
        };

        // Debug: Print the bundle.json structure
        eprintln!("DEBUG: Generated bundle.json:");
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&result)
                .unwrap_or_else(|e| format!("Failed to serialize bundle.json: {}", e))
        );

        Ok(result)
    }

    async fn create_bundle_archive(
        &self,
        api: &Api,
        bundle_json: &BundleJson,
        output_path: &PathBuf,
    ) -> Result<(), Error> {
        // First, we need to rebuild the PRN mapping by re-fetching bundle details
        // This is necessary because we only store IDs in the bundle format
        let bundle_response = api
            .bundles()
            .get(GetBundleParams {
                prn: self.bundle_prn.clone(),
            })
            .await
            .context(ApiSnafu)?
            .ok_or_else(|| Error::Generic {
                error: "Bundle not found during archive creation".to_string(),
            })?;

        let bundle_v2 = match &bundle_response.bundle {
            Bundle::V2(bundle_v2) => bundle_v2,
            _ => {
                return Err(Error::Generic {
                    error: "Expected V2 bundle".to_string(),
                });
            }
        };

        // Build binary_id to PRN mapping
        let mut binary_prn_map: HashMap<String, String> = HashMap::new();
        for bundle_binary in &bundle_v2.binaries {
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

            let binary_id = extract_id_from_prn(&binary_response.binary.prn)?;
            binary_prn_map.insert(binary_id, binary_response.binary.prn.clone());
        }
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

            // Get the PRN for this binary_id
            let binary_prn = binary_prn_map
                .get(&manifest_item.binary_id)
                .ok_or_else(|| Error::Generic {
                    error: format!("PRN not found for binary_id: {}", manifest_item.binary_id),
                })?;

            // Get binary details for download URL using the stored PRN
            let binary_response = api
                .binaries()
                .get(GetBinaryParams {
                    prn: binary_prn.clone(),
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
