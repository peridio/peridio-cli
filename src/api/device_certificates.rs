use std::fs;

use super::Command;
use crate::{print_json, ApiSnafu, Error};
use peridio_sdk::api::device_certificates::{
    CreateDeviceCertificateParams, DeleteDeviceCertificateParams, GetDeviceCertificateParams,
    ListDeviceCertificateParams,
};
use peridio_sdk::api::Api;
use snafu::ResultExt;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum DeviceCertificatesCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
    Get(Command<GetCommand>),
    List(Command<ListCommand>),
}

impl DeviceCertificatesCommand {
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Get(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct CreateCommand {
    #[structopt(long)]
    device_identifier: String,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(
        long,
        conflicts_with("certificate_path"),
        required_unless_one(&["certificate-path"])
    )]
    certificate: Option<String>,

    #[structopt(
        long,
        conflicts_with("certificate"),
        required_unless_one(&["certificate"])
    )]
    certificate_path: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self) -> Result<(), Error> {
        let certificate = if let Some(cert_path) = self.inner.certificate_path {
            fs::read_to_string(cert_path).unwrap()
        } else {
            self.inner.certificate.unwrap()
        };

        let encoded_certificate = base64::encode(&certificate);

        let params = CreateDeviceCertificateParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            device_identifier: self.inner.device_identifier,
            cert: encoded_certificate,
        };

        let api = Api::new(self.api_key, self.base_url);

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

#[derive(StructOpt, Debug)]
pub struct DeleteCommand {
    #[structopt(long)]
    device_identifier: String,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(long)]
    certificate_serial: String,
}

impl Command<DeleteCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = DeleteDeviceCertificateParams {
            device_identifier: self.inner.device_identifier,
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            certificate_serial: self.inner.certificate_serial,
        };

        let api = Api::new(self.api_key, self.base_url);

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

#[derive(StructOpt, Debug)]
pub struct GetCommand {
    #[structopt(long)]
    device_identifier: String,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,

    #[structopt(long)]
    certificate_serial: String,
}

impl Command<GetCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = GetDeviceCertificateParams {
            device_identifier: self.inner.device_identifier,
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            certificate_serial: self.inner.certificate_serial,
        };

        let api = Api::new(self.api_key, self.base_url);

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

#[derive(StructOpt, Debug)]
pub struct ListCommand {
    #[structopt(long)]
    device_identifier: String,

    #[structopt(long)]
    organization_name: String,

    #[structopt(long)]
    product_name: String,
}

impl Command<ListCommand> {
    async fn run(self) -> Result<(), Error> {
        let params = ListDeviceCertificateParams {
            organization_name: self.inner.organization_name,
            product_name: self.inner.product_name,
            device_identifier: self.inner.device_identifier,
        };

        let api = Api::new(self.api_key, self.base_url);

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
