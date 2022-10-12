use std::fs;
use std::path::PathBuf;

use super::Command;
use crate::{print_json, ApiSnafu, Error, FileSnafu, GlobalOptions};
use peridio_sdk::api::ca_certificates::{
    CreateCaCertificateParams, CreateVerificationCodeParams, DeleteCaCertificateParams,
    GetCaCertificateParams, ListCaCertificateParams,
};
use peridio_sdk::api::{Api, ApiOptions, CaCertificateJitp, UpdateCaCertificateParams};
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
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

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(parse(from_os_str), long, short = "c")]
    certificate_path: PathBuf,

    #[structopt(parse(from_os_str), long, short = "v")]
    verification_certificate_path: PathBuf,

    #[structopt(long)]
    description: Option<String>,

    #[structopt(long, requires_all = &["jitp-tags", "jitp-product-id"])]
    jitp_description: Option<String>,

    #[structopt(long, requires_all = &["jitp-description", "jitp-product-id"])]
    jitp_tags: Vec<String>,

    #[structopt(long, requires_all = &["jitp-tags", "jitp-description"])]
    jitp_product_name: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let cert = fs::read_to_string(self.inner.certificate_path).context(FileSnafu)?;
        let verification_cert =
            fs::read_to_string(self.inner.verification_certificate_path).context(FileSnafu)?;

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
            })
        } else {
            None
        };

        let params = CreateCaCertificateParams {
            organization_name: self.inner.organization_name,
            certificate: cert_base64,
            verification_certificate: verification_cert_base64,
            description: self.inner.description,
            jitp,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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

#[derive(StructOpt, Debug)]
pub struct DeleteCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    ca_certificate_serial: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteCaCertificateParams {
            organization_name: self.inner.organization_name,
            ca_certificate_serial: self.inner.ca_certificate_serial,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    ca_certificate_serial: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetCaCertificateParams {
            organization_name: self.inner.organization_name,
            ca_certificate_serial: self.inner.ca_certificate_serial,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
        });

        match api.ca_certificates().get(params).await.context(ApiSnafu)? {
            Some(ca_certificate) => print_json!(&ca_certificate),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct ListCommand {
    #[structopt(long)]
    organization_name: String,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListCaCertificateParams {
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
        });

        match api.ca_certificates().list(params).await.context(ApiSnafu)? {
            Some(ca_certificates) => print_json!(&ca_certificates),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct UpdateCommand {
    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    ca_certificate_serial: String,

    #[structopt(long)]
    description: Option<String>,

    #[structopt(long, conflicts_with_all = &["jitp-description", "jitp-tags", "jitp-product-name"])]
    disable_jitp: bool,

    #[structopt(long, requires_all = &["jitp-tags", "jitp-product-name"])]
    jitp_description: Option<String>,

    #[structopt(long, requires_all = &["jitp-description", "jitp-product-name"])]
    jitp_tags: Vec<String>,

    #[structopt(long, requires_all = &["jitp-tags", "jitp-description"])]
    jitp_product_name: Option<String>,
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
            }))
        } else {
            //do nothing
            None
        };

        let params = UpdateCaCertificateParams {
            organization_name: self.inner.organization_name,
            ca_certificate_serial: self.inner.ca_certificate_serial,
            description: self.inner.description,
            jitp,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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

#[derive(StructOpt, Debug)]
pub struct CreateVerificationCodeCommand {
    #[structopt(long)]
    organization_name: String,
}

impl Command<CreateVerificationCodeCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateVerificationCodeParams {
            organization_name: self.inner.organization_name,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key,
            endpoint: global_options.base_url,
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
