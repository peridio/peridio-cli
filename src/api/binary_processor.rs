use crate::api::binary_signatures;
use crate::api::binary_upload::BinaryUploader;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;

use backon::{ConstantBuilder, Retryable};
use console::style;
use peridio_sdk::api::binaries::{
    Binary, BinaryState, GetBinaryParams, GetBinaryResponse, UpdateBinaryParams,
    UpdateBinaryResponse,
};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use std::time::Duration;

/// Signature information for binary signing - supports both approaches
#[derive(Debug, Clone)]
pub struct SignatureConfig {
    /// The signing key identifier or PRN
    pub keyid: String,
    /// Pre-computed signature (bundles approach) - empty string means compute from signing config
    pub signature: String,
    /// Optional signing key pair name from config (traditional approach)
    pub signing_key_pair: Option<String>,
    /// Optional private key path (traditional approach)
    pub signing_key_private: Option<String>,
}

impl SignatureConfig {
    /// Create a pre-computed signature config (bundles approach)
    pub fn pre_computed(keyid: String, signature: String) -> Self {
        Self {
            keyid,
            signature,
            signing_key_pair: None,
            signing_key_private: None,
        }
    }

    /// Create a signing key pair config (traditional approach)
    pub fn from_key_pair(keyid: String, signing_key_pair: String) -> Self {
        Self {
            keyid,
            signature: String::new(), // Empty - will be computed
            signing_key_pair: Some(signing_key_pair),
            signing_key_private: None,
        }
    }

    /// Create a private key config (traditional approach)
    pub fn from_private_key(keyid: String, signing_key_private: String) -> Self {
        Self {
            keyid,
            signature: String::new(), // Empty - will be computed
            signing_key_pair: None,
            signing_key_private: Some(signing_key_private),
        }
    }

    /// Check if this signature needs computation
    pub fn needs_computation(&self) -> bool {
        self.signature.is_empty()
    }
}

/// Configuration for binary processing
#[derive(Debug, Clone)]
pub struct BinaryProcessorConfig {
    pub binary_part_size: Option<u64>,
    pub concurrency: Option<u8>,
    pub global_options: GlobalOptions,
    pub binary_content_hash: Option<String>,
    pub content_path: Option<String>,
    // Consolidated signatures approach - handles both traditional and bundles signing
    pub signatures: Vec<SignatureConfig>,
}

/// Shared binary processor that can handle state transitions
pub struct BinaryProcessor {
    config: BinaryProcessorConfig,
}

impl BinaryProcessor {
    pub fn new(config: BinaryProcessorConfig) -> Self {
        Self { config }
    }

    /// Process a binary through all possible state transitions
    /// This is the main entry point that mirrors the logic from binaries module
    pub async fn process_binary(
        &self,
        binary: &Binary,
        api: &Api,
        binary_content: Option<&[u8]>,
    ) -> Result<Binary, Error> {
        match binary.state {
            BinaryState::Uploadable => {
                let binary_content = binary_content.ok_or_else(|| Error::Generic {
                    error: "Binary content is required for Uploadable state".to_string(),
                })?;

                let binary = self
                    .process_binary_parts(binary, api, binary_content)
                    .await?;

                // After upload, continue with signing if available
                self.continue_processing_after_upload(binary, api).await
            }
            BinaryState::Hashable => {
                eprintln!("Updating binary to hashing...");
                let binary = self
                    .change_binary_status(BinaryState::Hashing, binary, api)
                    .await?;
                self.continue_processing_after_hashing(binary, api).await
            }
            BinaryState::Hashing => {
                self.continue_processing_after_hashing(binary.clone(), api)
                    .await
            }
            BinaryState::Signable => {
                if self.has_signing_config() {
                    self.sign_binary(binary, api).await
                } else {
                    Ok(binary.clone())
                }
            }
            BinaryState::Signed => {
                eprintln!("Binary is already signed");
                Ok(binary.clone())
            }
            _ => {
                eprintln!(
                    "Binary is in state {:?}, no processing available",
                    binary.state
                );
                Ok(binary.clone())
            }
        }
    }

    /// Continue processing after upload (handle signing if available)
    async fn continue_processing_after_upload(
        &self,
        binary: Binary,
        api: &Api,
    ) -> Result<Binary, Error> {
        if self.has_signing_config() {
            eprintln!("Waiting for cloud hashing...");
            let binary = self.wait_for_signable_state(&binary, api).await?;
            self.sign_binary(&binary, api).await
        } else {
            Ok(binary)
        }
    }

    /// Continue processing after hashing transition
    async fn continue_processing_after_hashing(
        &self,
        binary: Binary,
        api: &Api,
    ) -> Result<Binary, Error> {
        if self.has_signing_config() {
            eprintln!("Waiting for cloud hashing...");
            let binary = self.wait_for_signable_state(&binary, api).await?;
            self.sign_binary(&binary, api).await
        } else {
            Ok(binary)
        }
    }

    /// Process binary parts upload (mirrors process_binary_parts from binaries module)
    async fn process_binary_parts(
        &self,
        binary: &Binary,
        api: &Api,
        binary_content: &[u8],
    ) -> Result<Binary, Error> {
        eprintln!("Evaluating binary parts...");

        let mut uploader = BinaryUploader::new();

        // Configure uploader with options
        if let Some(part_size) = self.config.binary_part_size {
            uploader = uploader.with_part_size(part_size);
        }

        if let Some(concurrency) = self.config.concurrency {
            uploader = uploader.with_concurrency(concurrency);
        }

        uploader
            .upload_from_memory(
                binary,
                api,
                self.config.global_options.clone(),
                binary_content,
            )
            .await?;

        eprintln!("Validating Upload");

        // After upload, transition to Hashable then Hashing
        let binary = self
            .change_binary_status(BinaryState::Hashable, binary, api)
            .await?;
        let binary = self
            .change_binary_status(BinaryState::Hashing, &binary, api)
            .await?;

        Ok(binary)
    }

    /// Wait for binary to reach Signable state (mirrors check_for_state_change)
    async fn wait_for_signable_state(&self, binary: &Binary, api: &Api) -> Result<Binary, Error> {
        (|| async {
            let params = GetBinaryParams {
                prn: binary.prn.clone(),
            };

            match api.binaries().get(params).await.context(ApiSnafu)? {
                Some(GetBinaryResponse { binary }) => {
                    if matches!(binary.state, BinaryState::Signable) {
                        Ok(binary)
                    } else {
                        Err(Error::Generic {
                            error: "Binary not yet signable".to_string(),
                        })
                    }
                }
                None => Err(Error::Generic {
                    error: "Binary not found during state check".to_string(),
                }),
            }
        })
        .retry(
            &ConstantBuilder::default()
                .with_delay(Duration::new(10, 0))
                .with_max_times(30),
        )
        .await
    }

    /// Sign the binary with multiple signatures (consolidated approach)
    async fn sign_binary(&self, binary: &Binary, api: &Api) -> Result<Binary, Error> {
        if !self.config.signatures.is_empty() {
            let mut successful_signatures = 0;
            let total_signatures = self.config.signatures.len();
            let mut failed_keyids = Vec::new();

            for sig_config in self.config.signatures.iter() {
                if sig_config.needs_computation() {
                    // Compute signature using signing configuration
                    let command = crate::api::binary_signatures::CreateCommand {
                        binary_prn: binary.prn.clone(),
                        binary_content_path: self.config.content_path.clone(),
                        signature: None,
                        signing_key_pair: sig_config.signing_key_pair.clone(),
                        signing_key_private: sig_config.signing_key_private.clone(),
                        signing_key_prn: Some(sig_config.keyid.clone()),
                        api: Some(api.clone()),
                        binary_content_hash: self.config.binary_content_hash.clone(),
                    };

                    match command.run(self.config.global_options.clone()).await {
                        Ok(_) => successful_signatures += 1,
                        Err(_) => failed_keyids.push(sig_config.keyid.clone()),
                    }
                } else {
                    // Check if signature already exists before creating
                    match binary_signatures::signature_exists(api, &binary.prn, &sig_config.keyid)
                        .await
                    {
                        Ok(true) => {
                            // Signature already exists, treat as success
                            successful_signatures += 1;
                        }
                        Ok(false) => {
                            // Signature doesn't exist, create it
                            let params =
                                peridio_sdk::api::binary_signatures::CreateBinarySignatureParams {
                                    binary_prn: binary.prn.clone(),
                                    signing_key_prn: None,
                                    signature: sig_config.signature.clone(),
                                    signing_key_keyid: Some(sig_config.keyid.clone()),
                                };

                            match api
                                .binary_signatures()
                                .create(params)
                                .await
                                .context(crate::ApiSnafu)
                            {
                                Ok(_) => {
                                    successful_signatures += 1;
                                }
                                Err(_) => {
                                    failed_keyids.push(sig_config.keyid.clone());
                                }
                            }
                        }
                        Err(_) => {
                            // Error checking existence, add to failed list
                            failed_keyids.push(sig_config.keyid.clone());
                        }
                    }
                }
            }

            // Return error if any signatures failed
            if successful_signatures < total_signatures {
                return Err(Error::Generic {
                    error: format!(
                        "Failed to create signatures for binary (failed keyids: {})",
                        style(failed_keyids.join(", ")).magenta()
                    ),
                });
            }
        }
        // No fallback needed - all signing should go through signatures field

        // After processing signatures (created or verified existing), transition binary to Signed state
        // This ensures the binary is marked as signed whether we created new signatures or found existing ones
        let binary = self
            .change_binary_status(BinaryState::Signed, binary, api)
            .await?;

        Ok(binary)
    }

    /// Change binary status (mirrors change_binary_status from binaries module)
    async fn change_binary_status(
        &self,
        state: BinaryState,
        binary: &Binary,
        api: &Api,
    ) -> Result<Binary, Error> {
        let params = UpdateBinaryParams {
            prn: binary.prn.clone(),
            custom_metadata: None,
            description: None,
            state: Some(state),
            hash: None,
            size: None,
        };

        match api.binaries().update(params).await.context(ApiSnafu)? {
            Some(UpdateBinaryResponse { binary }) => Ok(binary),
            None => Err(Error::Generic {
                error: "Failed to update binary status".to_string(),
            }),
        }
    }

    /// Check if signing configuration is available
    fn has_signing_config(&self) -> bool {
        !self.config.signatures.is_empty()
    }
}
