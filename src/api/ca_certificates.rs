use super::Command;
use crate::utils::list::ListArgs;
use crate::{print_json, ApiSnafu, Error, GlobalOptions, NonExistingPathSnafu};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use peridio_sdk::api::ca_certificates::CreateCaCertificateParams;
use peridio_sdk::api::ca_certificates::CreateVerificationCodeParams;
use peridio_sdk::api::ca_certificates::DeleteCaCertificateParams;
use peridio_sdk::api::ca_certificates::GetCaCertificateParams;
use peridio_sdk::api::ca_certificates::ListCaCertificateParams;
use peridio_sdk::api::ca_certificates::UpdateCaCertificateParams;
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
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

        let params = CreateCaCertificateParams {
            certificate: cert_base64,
            verification_certificate: verification_cert_base64,
            description: self.inner.description,
        };

        let api = Api::from(global_options);

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
    /// The prn of the CA certificate to delete.
    #[arg(long)]
    prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteCaCertificateParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

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
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetCaCertificateParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        match api.ca_certificates().get(params).await.context(ApiSnafu)? {
            Some(ca_certificate) => print_json!(&ca_certificate),
            None => panic!(),
        }

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
        let params = ListCaCertificateParams {
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

        match api.ca_certificates().list(params).await.context(ApiSnafu)? {
            Some(ca_certificates) => print_json!(&ca_certificates),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// The prn of the CA certificate to update.
    #[arg(long)]
    prn: String,

    /// An arbitrary string attached to the resource. Often useful for displaying to users.
    #[arg(long)]
    description: Option<String>,
}

impl Command<UpdateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = UpdateCaCertificateParams {
            prn: self.inner.prn,
            description: self.inner.description,
        };

        let api = Api::from(global_options);

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
        let params = CreateVerificationCodeParams {};

        let api = Api::from(global_options);

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
