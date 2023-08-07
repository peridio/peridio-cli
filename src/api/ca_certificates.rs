use std::fs;
use std::path::PathBuf;

use super::Command;
use crate::{print_json, ApiSnafu, Error, GlobalOptions, NonExistingPathSnafu};
use clap::Parser;
use peridio_sdk::api::ca_certificates::{
    CreateCaCertificateParams, CreateVerificationCodeParams, DeleteCaCertificateParams,
    GetCaCertificateParams, ListCaCertificateParams,
};
use peridio_sdk::api::{Api, ApiOptions, CaCertificateJitp, UpdateCaCertificateParams};
use snafu::ResultExt;

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
    #[arg(long, short = 'c')]
    certificate_path: PathBuf,

    #[arg(long, short = 'v')]
    verification_certificate_path: PathBuf,

    #[arg(long)]
    description: Option<String>,

    #[arg(long, requires_all = &["jitp_tags", "jitp_product_name"])]
    jitp_description: Option<String>,

    #[arg(long, requires_all = &["jitp_description", "jitp_product_name"])]
    jitp_tags: Vec<String>,

    #[arg(long, requires_all = &["jitp_tags", "jitp_description"])]
    jitp_product_name: Option<String>,

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

        let cert_base64 = base64::encode(cert);
        let verification_cert_base64 = base64::encode(verification_cert);

        let jitp = if let (Some(description), true, Some(product_name)) = (
            self.inner.jitp_description,
            !self.inner.jitp_tags.is_empty(),
            self.inner.jitp_product_name,
        ) {
            Some(CaCertificateJitp {
                description,
                tags: self.inner.jitp_tags,
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
    #[arg(long)]
    ca_certificate_serial: String,

    #[arg(long)]
    description: Option<String>,

    #[arg(long, conflicts_with_all = &["jitp_description", "jitp_tags", "jitp_product_name"])]
    disable_jitp: bool,

    #[arg(long, requires_all = &["jitp_tags", "jitp_product_name"])]
    jitp_description: Option<String>,

    #[arg(long, requires_all = &["jitp_description", "jitp_product_name"])]
    jitp_tags: Vec<String>,

    #[arg(long, requires_all = &["jitp_tags", "jitp_description"])]
    jitp_product_name: Option<String>,

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
