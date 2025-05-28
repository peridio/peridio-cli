use super::Command;
use crate::print_json;
use crate::utils::list::ListArgs;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use peridio_sdk::api::device_certificates::CreateDeviceCertificateParams;
use peridio_sdk::api::device_certificates::DeleteDeviceCertificateParams;
use peridio_sdk::api::device_certificates::GetDeviceCertificateParams;
use peridio_sdk::api::device_certificates::ListDeviceCertificateParams;
use peridio_sdk::api::Api;
use peridio_sdk::list_params::ListParams;
use snafu::ResultExt;
use std::fs;

#[derive(Parser, Debug)]
pub enum DeviceCertificatesCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
}

impl DeviceCertificatesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
            Self::Get(cmd) => cmd.run(global_options).await,
            Self::List(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The prn of the device you wish to create a certificate for.
    #[arg(long)]
    device_prn: String,

    /// The certificate PEM content.
    #[arg(
        long,
        conflicts_with("certificate_path"),
        required_unless_present("certificate_path")
    )]
    certificate: Option<String>,

    /// The path to the certificate's PEM content
    #[arg(
        long,
        conflicts_with("certificate"),
        required_unless_present("certificate")
    )]
    certificate_path: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let certificate = if let Some(cert_path) = self.inner.certificate_path {
            fs::read_to_string(cert_path).unwrap()
        } else {
            self.inner.certificate.unwrap()
        };

        let encoded_certificate = general_purpose::STANDARD.encode(&certificate);

        let params = CreateDeviceCertificateParams {
            device_prn: self.inner.device_prn,
            certificate: encoded_certificate,
        };

        let api = Api::from(global_options);

        match api
            .device_certificates()
            .create(params)
            .await
            .context(ApiSnafu)?
        {
            Some(device_certificate) => print_json!(&device_certificate),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    /// The prn of the device_certificate.
    #[arg(long)]
    prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteDeviceCertificateParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        if (api
            .device_certificates()
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
    /// The prn of the device_certificate.
    #[arg(long)]
    prn: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetDeviceCertificateParams {
            prn: self.inner.prn,
        };

        let api = Api::from(global_options);

        match api
            .device_certificates()
            .get(params)
            .await
            .context(ApiSnafu)?
        {
            Some(device_certificate) => print_json!(&device_certificate),
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
        let params = ListDeviceCertificateParams {
            list: ListParams::from(self.inner.list_args),
        };

        let api = Api::from(global_options);

        match api
            .device_certificates()
            .list(params)
            .await
            .context(ApiSnafu)?
        {
            Some(device_certificates) => print_json!(&device_certificates),
            None => panic!(),
        }

        Ok(())
    }
}
