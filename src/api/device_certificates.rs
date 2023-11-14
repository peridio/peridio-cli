use super::Command;
use crate::print_json;
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
use peridio_sdk::api::ApiOptions;
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
    /// The identifier of the device you wish to create a certificate for.
    #[arg(long)]
    device_identifier: String,

    /// The name of the product you wish to create the resource within.
    #[arg(long)]
    product_name: String,

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
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            device_identifier: self.inner.device_identifier,
            cert: encoded_certificate,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

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
    /// The identifier of the device you wish to delete a certificate for.
    #[arg(long)]
    device_identifier: String,

    /// The name of the product you wish to delete the resource within.
    #[arg(long)]
    product_name: String,

    /// The serial number of the certificate you wish to delete.
    #[arg(long)]
    certificate_serial: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteDeviceCertificateParams {
            device_identifier: self.inner.device_identifier,
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            certificate_serial: self.inner.certificate_serial,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

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
    /// The identifier of the device you wish to get a certificate for.
    #[arg(long)]
    device_identifier: String,

    /// The name of the product you wish to get the resource within.
    #[arg(long)]
    product_name: String,

    /// The serial number of the certificate you wish to get.
    #[arg(long)]
    certificate_serial: String,
}

impl Command<GetCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = GetDeviceCertificateParams {
            device_identifier: self.inner.device_identifier,
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            certificate_serial: self.inner.certificate_serial,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

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
    /// The identifier of the device you wish to list certificates for.
    #[arg(long)]
    device_identifier: String,

    /// The name of the product you wish to list the resource within.
    #[arg(long)]
    product_name: String,
}

impl Command<ListCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = ListDeviceCertificateParams {
            organization_name: global_options.organization_name.unwrap(),
            product_name: self.inner.product_name,
            device_identifier: self.inner.device_identifier,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .device_certificates()
            .list(params)
            .await
            .context(ApiSnafu)?
        {
            Some(device_certificate) => print_json!(&device_certificate),
            None => panic!(),
        }

        Ok(())
    }
}
