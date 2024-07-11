use super::Command;
use crate::{print_json, ApiSnafu, Error, GlobalOptions, NonExistingPathSnafu};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use peridio_sdk::api::ca_certificates::CaCertificateJitp;
use peridio_sdk::api::ca_certificates::CreateCaCertificateParams;
use peridio_sdk::api::ca_certificates::CreateVerificationCodeParams;
use peridio_sdk::api::ca_certificates::DeleteCaCertificateParams;
use peridio_sdk::api::ca_certificates::GetCaCertificateParams;
use peridio_sdk::api::ca_certificates::ListCaCertificateParams;
use peridio_sdk::api::ca_certificates::UpdateCaCertificateParams;
use peridio_sdk::api::{Api, ApiOptions};
use snafu::ResultExt;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub enum CaCertificatesCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
    Update(Command<UpdateCommand>),
    CreateVerificationCode(Command<CreateVerificationCodeCommand>),
}

impl CaCertificatesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
            Self::Update(cmd) => cmd.run(global_options).await,
            Self::CreateVerificationCode(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The path of the CA certificate to create.
    #[arg(long, short = 'c')]
    certificate_path: PathBuf,

    /// The path of the verification certificate.
    #[arg(long, short = 'v')]
    verification_certificate_path: PathBuf,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,

    /// An arbitrary string attached to the jitp resource. Often useful for displaying to users.
    #[arg(long, requires_all = &["jitp_tags", "jitp_product_name"])]
    jitp_description: Option<String>,

    /// Tags that will be automatically applied to devices that JITP with this CA certificate.
    #[arg(long, requires_all = &["jitp_description", "jitp_product_name"], num_args = 0.., value_delimiter = ',')]
    jitp_tags: Vec<String>,

    /// The target that will be automatically applied to devices that JITP with this CA certificate.
    #[arg(long)]
    jitp_target: Option<String>,

    /// The product that will be automatically applied to devices that JITP with this CA certificate.
    #[arg(long, requires_all = &["jitp_tags", "jitp_description"])]
    jitp_product_name: Option<String>,

    /// The cohort that will be automatically applied to devices that JITP with this CA certificate.
    #[arg(long)]
    jitp_cohort_prn: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let cert =
            fs::read_to_string(&self.inner.certificate_path).context(NonExistingPathSnafu {
                path: &self.inner.certificate_path,
            })?;
        let verification_cert = fs::read_to_string(self.inner.verification_certificate_path)
            .context(NonExistingPathSnafu {
                path: &self.inner.certificate_path,
            })?;

        let cert_base64 = general_purpose::STANDARD.encode(cert);
        let verification_cert_base64 = general_purpose::STANDARD.encode(verification_cert);

        let jitp = if let (Some(description), true, Some(product_name)) = (
            self.inner.jitp_description,
            !self.inner.jitp_tags.is_empty(),
            self.inner.jitp_product_name,
        ) {
            Some(CaCertificateJitp {
                description,
                tags: self.inner.jitp_tags,
                target: self.inner.jitp_target,
                product_name,
                cohort_prn: self.inner.jitp_cohort_prn,
            })
        } else {
            None
        };

        let params = CreateCaCertificateParams {
            organization_name: global_options.organization_name.unwrap(),
            certificate: cert_base64,
            verification_certificate: verification_cert_base64,
            description: self.inner.description,
            jitp,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .ca_certificates()
            .create(params)
            .await
            .context(ApiSnafu)?
        {
            Some(ca_certificate) => print_json!(&ca_certificate),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The serial of the CA certificate to delete.
    #[arg(long)]
    ca_certificate_serial: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteCaCertificateParams {
            organization_name: global_options.organization_name.unwrap(),
            ca_certificate_serial: self.inner.ca_certificate_serial,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        if (api
            .ca_certificates()
            .delete(params)
            .await
            .context(ApiSnafu)?)
        .is_some()
        {
            panic!()
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct GetCommand {
    #[arg(long)]
    ca_certificate_serial: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetCaCertificateParams {
            organization_name: global_options.organization_name.unwrap(),
            ca_certificate_serial: self.inner.ca_certificate_serial,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.ca_certificates().get(params).await.context(ApiSnafu)? {
            Some(ca_certificate) => print_json!(&ca_certificate),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct ListCommand {}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListCaCertificateParams {
            organization_name: global_options.organization_name.unwrap(),
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api.ca_certificates().list(params).await.context(ApiSnafu)? {
            Some(ca_certificates) => print_json!(&ca_certificates),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// The serial of the CA certificate to update.
    #[arg(long)]
    ca_certificate_serial: String,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,

    /// Pass this option to disable JITP for this CA certificate.
    #[arg(long, conflicts_with_all = &["jitp_description", "jitp_tags", "jitp_product_name"])]
    disable_jitp: bool,

    /// An arbitrary string attached to the jitp resource. Often useful for displaying to users.
    #[arg(long, requires_all = &["jitp_tags", "jitp_product_name"])]
    jitp_description: Option<String>,

    /// Tags that will be automatically applied to devices that JITP with this CA certificate.
    #[arg(long, requires_all = &["jitp_description", "jitp_product_name"], num_args = 0.., value_delimiter = ',')]
    jitp_tags: Vec<String>,

    /// The target that will be automatically applied to devices that JITP with this CA certificate.
    #[arg(long)]
    jitp_target: Option<String>,

    /// The product that will be automatically applied to devices that JITP with this CA certificate.
    #[arg(long, requires_all = &["jitp_tags", "jitp_description"])]
    jitp_product_name: Option<String>,

    /// The cohort that will be automatically applied to devices that JITP with this CA certificate.
    #[arg(long)]
    jitp_cohort_prn: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let jitp = if self.inner.disable_jitp {
            // disable jitp
            Some(None)
        } else if let (Some(description), true, Some(product_name)) = (
            self.inner.jitp_description,
            !self.inner.jitp_tags.is_empty(),
            self.inner.jitp_product_name,
        ) {
            Some(Some(CaCertificateJitp {
                description,
                tags: self.inner.jitp_tags,
                target: self.inner.jitp_target,
                product_name,
                cohort_prn: self.inner.jitp_cohort_prn,
            }))
        } else {
            //do nothing
            None
        };

        let params = UpdateCaCertificateParams {
            organization_name: global_options.organization_name.unwrap(),
            ca_certificate_serial: self.inner.ca_certificate_serial,
            description: self.inner.description,
            jitp,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .ca_certificates()
            .update(params)
            .await
            .context(ApiSnafu)?
        {
            Some(ca_certificate) => print_json!(&ca_certificate),
            None => panic!(),
        }

        Ok(())
    }
}

/// Create a verification code for use in creating a CA certificate.
///
/// This command is used to create a verification code that can be used to create a CA certificate.
#[derive(Parser, Debug)]
pub struct CreateVerificationCodeCommand {}

impl Command<CreateVerificationCodeCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateVerificationCodeParams {
            organization_name: global_options.organization_name.unwrap(),
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .ca_certificates()
            .create_verification_code(params)
            .await
            .context(ApiSnafu)?
        {
            Some(verification_code) => print_json!(&verification_code),
            None => panic!(),
        }

        Ok(())
    }
}
