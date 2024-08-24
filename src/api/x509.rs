use super::Command;
use crate::print_json;
use crate::utils::{Style, StyledStr};
use crate::{
    CertParamsCreationSnafu, CertificateCreationSnafu, Error, GlobalOptions, NonExistingPathSnafu,
};
use ::time::macros::format_description;
use ::time::OffsetDateTime;
use clap::Parser;
use rcgen::{CertificateParams, DistinguishedName, DnType, IsCa, KeyPair};
use serde_json::json;
use snafu::ResultExt;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct CreateCommand {
    /// The Common Name (CN) for the certificate
    #[arg(long)]
    common_name: String,

    /// Whether this certificate is a Certificate Authority (CA)
    #[arg(long, default_value = "false")]
    is_ca: bool,

    /// The start date of the certificate's validity period (format: YYYY-MM-DD)
    #[arg(long)]
    start_date: String,

    /// The end date of the certificate's validity period (format: YYYY-MM-DD)
    #[arg(long)]
    end_date: String,

    /// Path to the private key file of the signer (required if signer_cert is provided)
    #[arg(long, requires = "signer_cert", conflicts_with = "signer")]
    signer_key: Option<PathBuf>,

    /// Path to the certificate file of the signer (required if signer_key is provided)
    #[arg(long, requires = "signer_key", conflicts_with = "signer")]
    signer_cert: Option<PathBuf>,

    /// Directory to save the created certificate and private key to (defaults to current working directory)
    #[arg(long)]
    out: Option<PathBuf>,

    /// The name of a certificate authority in your Peridio CLI config.
    #[arg(long, conflicts_with_all = ["signer_key", "signer_cert"])]
    signer: Option<String>,
}

#[derive(Parser, Debug)]
pub enum X509Command {
    Create(Command<CreateCommand>),
}

impl X509Command {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
        }
    }
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let mut params = CertificateParams::default();

        // is ca
        params.is_ca = if self.inner.is_ca {
            IsCa::Ca(rcgen::BasicConstraints::Unconstrained)
        } else {
            IsCa::NoCa
        };

        // authority key identifier
        params.use_authority_key_identifier_extension = true;

        // distinguished name
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, self.inner.common_name.clone());
        params.distinguished_name = distinguished_name;

        //  key usages
        params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];

        // extended key usages
        params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ClientAuth];

        // validity period
        let start = parse_date(&self.inner.start_date)?;
        let end = parse_date(&self.inner.end_date)?;
        params.not_before = start;
        params.not_after = end;

        // key pair
        let key_pair = KeyPair::generate().context(CertParamsCreationSnafu)?;

        // signed by or self signed
        let cert = if let Some(signer_name) = self.inner.signer {
            if let Some(certificate_authorities) = global_options.certificate_authorities {
                if let Some(signer) = certificate_authorities.get(&signer_name) {
                    let signer_key = {
                        let path = Path::new(&signer.private_key);
                        if !path.exists() {
                            return Err(Error::NonExistingPath {
                                path: path.to_path_buf(),
                                source: std::io::Error::new(
                                    std::io::ErrorKind::NotFound,
                                    "File not found",
                                ),
                            });
                        }
                        KeyPair::from_pem(
                            &fs::read_to_string(path).context(NonExistingPathSnafu { path })?,
                        )
                        .context(CertParamsCreationSnafu)?
                    };

                    let signer_cert_pem = {
                        let path = Path::new(&signer.certificate);
                        if !path.exists() {
                            return Err(Error::NonExistingPath {
                                path: path.to_path_buf(),
                                source: std::io::Error::new(
                                    std::io::ErrorKind::NotFound,
                                    "File not found",
                                ),
                            });
                        }
                        fs::read_to_string(path).context(NonExistingPathSnafu { path })?
                    };

                    let signer_cert = CertificateParams::from_ca_cert_pem(&signer_cert_pem)
                        .context(CertParamsCreationSnafu)?
                        .self_signed(&signer_key)
                        .context(CertificateCreationSnafu)?;
                    params
                        .signed_by(&key_pair, &signer_cert, &signer_key)
                        .context(CertificateCreationSnafu)?
                } else {
                    let mut error = StyledStr::new();
                    error.push_str(Some(Style::Error), "error: ".to_string());
                    error.push_str(None, "Config file field ".to_string());
                    error.push_str(None, "'".to_string());
                    error.push_str(
                        Some(Style::Warning),
                        format!("certificate_authorities.{signer_name}").to_string(),
                    );
                    error.push_str(None, "'".to_string());
                    error.push_str(
                        None,
                        " is unset or null, but is required by the --signer option.".to_string(),
                    );
                    error.print_data_err();
                }
            } else {
                let mut error = StyledStr::new();
                error.push_str(Some(Style::Error), "error: ".to_string());
                error.push_str(None, "Config file field ".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(Some(Style::Warning), "certificate_authorities".to_string());
                error.push_str(None, "'".to_string());
                error.push_str(
                    None,
                    " is unset or null, but is required by the --signer option.".to_string(),
                );
                error.print_data_err();
            }
        } else if let (Some(signer_key_path), Some(signer_cert_path)) =
            (self.inner.signer_key, self.inner.signer_cert)
        {
            let signer_key = {
                let path = Path::new(&signer_key_path);
                if !path.exists() {
                    return Err(Error::NonExistingPath {
                        path: path.to_path_buf(),
                        source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
                    });
                }
                KeyPair::from_pem(&fs::read_to_string(path).context(NonExistingPathSnafu { path })?)
                    .context(CertParamsCreationSnafu)?
            };

            let signer_cert_pem = {
                let path = Path::new(&signer_cert_path);
                if !path.exists() {
                    return Err(Error::NonExistingPath {
                        path: path.to_path_buf(),
                        source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
                    });
                }
                fs::read_to_string(path).context(NonExistingPathSnafu { path })?
            };

            let signer_cert = CertificateParams::from_ca_cert_pem(&signer_cert_pem)
                .context(CertParamsCreationSnafu)?
                .self_signed(&signer_key)
                .context(CertificateCreationSnafu)?;
            params
                .signed_by(&key_pair, &signer_cert, &signer_key)
                .context(CertificateCreationSnafu)?
        } else {
            params
                .self_signed(&key_pair)
                .context(CertificateCreationSnafu)?
        };

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let out_dir = self
            .inner
            .out
            .unwrap_or_else(|| env::current_dir().unwrap());
        fs::create_dir_all(&out_dir).unwrap();
        let cert_filename = format!("{}-certificate.pem", self.inner.common_name);
        let key_filename = format!("{}-private-key.pem", self.inner.common_name);
        fs::write(out_dir.join(cert_filename.clone()), &cert_pem).unwrap();
        fs::write(out_dir.join(key_filename.clone()), &key_pem).unwrap();

        print_json!(&json!({
            "certificate": out_dir.join(cert_filename),
            "private_key": out_dir.join(key_filename)
        }));

        Ok(())
    }
}

fn parse_date(date_str: &str) -> Result<OffsetDateTime, Error> {
    let format = format_description!("[year]-[month]-[day]");
    time::Date::parse(date_str, &format)
        .map(|date| date.with_time(time::Time::MIDNIGHT).assume_utc())
        .map_err(|e| Error::DateParse { source: e })
}
