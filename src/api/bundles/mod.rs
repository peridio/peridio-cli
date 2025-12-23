mod push;

use super::Command;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::utils::PRNType;
use crate::utils::PRNValueParser;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
use peridio_sdk::api::bundles::{
    Bundle, CreateBundleBinary, CreateBundleParams, CreateBundleParamsV1, CreateBundleParamsV2,
    DeleteBundleParams, GetBundleParams, ListBundlesParams, UpdateBundleParams,
};
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
use serde_json::{Map, Value};
use snafu::ResultExt;

pub use push::PushCommand;

// Trait to add helper methods to Bundle enum
trait BundleExt {
    fn print_json(&self) -> Result<(), Error>;
}

impl BundleExt for Bundle {
    fn print_json(&self) -> Result<(), Error> {
        match self {
            Bundle::V1(bundle_v1) => print_json!(&bundle_v1),
            Bundle::V2(bundle_v2) => print_json!(&bundle_v2),
        }
        Ok(())
    }
}

// Helper function to create version-specific CreateBundleParams
fn create_bundle_params_v1(
    artifact_version_prns: Vec<String>,
    id: Option<String>,
    name: Option<String>,
) -> Result<CreateBundleParams, Error> {
    Ok(CreateBundleParams::V1(CreateBundleParamsV1 {
        artifact_version_prns,
        id,
        name,
    }))
}

fn create_bundle_params_v2(
    binaries: Vec<CreateBundleBinary>,
    id: Option<String>,
    name: Option<String>,
) -> Result<CreateBundleParams, Error> {
    Ok(CreateBundleParams::V2(CreateBundleParamsV2 {
        binaries,
        id,
        name,
    }))
}

#[derive(Parser, Debug)]
pub enum BundlesCommand {
    Create(Command<CreateCommand>),
    Push(Command<PushCommand>),
    List(Command<ListCommand>),
    Get(Command<GetCommand>),
    Update(Command<UpdateCommand>),
    Delete(Command<DeleteCommand>),
}

impl BundlesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Push(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The PRNs of the artifact versions to include binaries from in the bundle (API v1 only).
    ///
    /// Values can be provided by passing each value in a flag
    /// or by delimiting all values with ","
    #[clap(short, long, value_parser, num_args = 1.., value_delimiter = ',')]
    artifact_version_prns: Option<Vec<String>>,

    /// The binaries to include in the bundle (API v2 only).
    /// Format: "prn=prn_value[;custom_metadata={json|null}]" where prn is required and custom_metadata is optional.
    /// custom_metadata can be: missing (no metadata), null (explicit null), or JSON object.
    /// Example: --binaries 'prn=prn:1:org:binary:id;custom_metadata={"version":"1.0"}'
    /// Example: --binaries 'prn=prn:1:org:binary:id;custom_metadata=null'
    /// Example: --binaries 'prn=prn:1:org:binary:id' (without metadata)
    #[arg(long, value_parser, num_args = 1..)]
    binaries: Option<Vec<String>>,

    /// A user provided custom UUID id for the bundle database record.
    #[arg(long)]
    id: Option<String>,

    /// The name of the bundle.
    #[arg(long)]
    name: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let api_version = global_options.api_version.unwrap_or(2);
        let api = Api::from(global_options);

        let params = match api_version {
            1 => {
                // Check if user provided v2-only options
                if self.inner.binaries.is_some() {
                    return Err(Error::Generic {
                        error: "The --binaries option is only supported in API version 2. Use --artifact-version-prns for API version 1 or use --api-version 2".to_string(),
                    });
                }

                // Check if artifact_version_prns is provided for v1
                let artifact_version_prns =
                    self.inner
                        .artifact_version_prns
                        .ok_or_else(|| Error::Generic {
                            error: "API version 1 requires --artifact-version-prns to be specified"
                                .to_string(),
                        })?;

                create_bundle_params_v1(artifact_version_prns, self.inner.id, self.inner.name)?
            }
            2 => {
                // Check if user provided v1-only options
                if self.inner.artifact_version_prns.is_some() {
                    return Err(Error::Generic {
                        error: "The --artifact-version-prns option is only supported in API version 1. Use --binaries for API version 2 or use --api-version 1".to_string(),
                    });
                }

                // Check if binaries is provided for v2
                let binaries_input = self.inner.binaries.clone().ok_or_else(|| Error::Generic {
                    error: "API version 2 requires --binaries to be specified".to_string(),
                })?;

                let binaries = self.parse_binaries_from_input(binaries_input)?;
                create_bundle_params_v2(binaries, self.inner.id, self.inner.name)?
            }
            _ => {
                return Err(Error::Generic {
                    error: format!("Unsupported API version: {}", api_version),
                });
            }
        };

        match api.bundles().create(params).await.context(ApiSnafu)? {
            Some(response) => response.bundle.print_json()?,
            None => panic!(),
        }

        Ok(())
    }

    fn parse_binaries_from_input(
        &self,
        binaries_input: Vec<String>,
    ) -> Result<Vec<CreateBundleBinary>, Error> {
        let mut binaries = Vec::new();

        for binary_spec in binaries_input {
            let mut prn: Option<String> = None;
            let mut custom_metadata: Option<Map<String, Value>> = None;

            // Parse key=value pairs
            for pair in binary_spec.split(';') {
                let parts: Vec<&str> = pair.splitn(2, '=').collect();
                if parts.len() != 2 {
                    return Err(Error::Generic {
                        error: format!(
                            "Invalid binary format: '{}'. Expected 'key=value' pairs separated by semicolons. Required: prn=value. Optional: custom_metadata={{json|null}}. Example: 'prn=prn:1:org:artifact_version:id;custom_metadata={{\"version\":\"1.0\"}}' or 'prn=prn:1:org:artifact_version:id;custom_metadata=null'",
                            pair
                        ),
                    });
                }

                let key = parts[0].trim();
                let value = parts[1].trim();

                match key {
                    "prn" => {
                        prn = Some(value.to_string());
                    }
                    "custom_metadata" => {
                        if value == "null" {
                            // Explicitly set to null - same as missing (None)
                            custom_metadata = None;
                        } else {
                            // Parse as JSON object
                            let metadata: Map<String, Value> =
                                serde_json::from_str(value).map_err(|e| Error::Generic {
                                    error: format!("Invalid JSON in custom_metadata: {}. Use 'null' for explicit null or valid JSON object.", e),
                                })?;
                            custom_metadata = Some(metadata);
                        }
                    }
                    _ => {
                        return Err(Error::Generic {
                            error: format!(
                                "Unknown key '{}'. Supported keys: 'prn' (required), 'custom_metadata' (optional: JSON object or 'null')",
                                key
                            ),
                        });
                    }
                }
            }

            let prn = prn.ok_or_else(|| Error::Generic {
                error: "Missing required 'prn' key in binary specification".to_string(),
            })?;

            binaries.push(CreateBundleBinary {
                prn,
                custom_metadata,
            });
        }

        Ok(binaries)
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The PRN of the resource to delete.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteBundleParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        api.bundles().delete(params).await.context(ApiSnafu)?;

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
        let params = ListBundlesParams {
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

        match api.bundles().list(params).await.context(ApiSnafu)? {
            Some(bundles_response) => print_json!(&bundles_response),
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
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetBundleParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        match api.bundles().get(params).await.context(ApiSnafu)? {
            Some(response) => response.bundle.print_json()?,
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// The PRN of the resource to update.
    #[arg(
        long,
        value_parser = PRNValueParser::new(PRNType::Bundle)
    )]
    prn: String,

    /// The resource's name, meant to be displayable to users.
    #[arg(long)]
    pub name: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateBundleParams {
            prn: self.inner.prn,
            name: self.inner.name,
        };

        let api = Api::from(global_options);

        match api.bundles().update(params).await.context(ApiSnafu)? {
            Some(response) => response.bundle.print_json()?,
            None => panic!(),
        }

        Ok(())
    }
}
