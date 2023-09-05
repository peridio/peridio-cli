use super::Command;
use crate::print_json;
use crate::ApiSnafu;
use crate::Error;
use crate::GlobalOptions;
use clap::Parser;
// use ed25519_dalek::pkcs8::DecodePrivateKey;
// use ed25519_dalek::pkcs8::DecodePublicKey;
// use ed25519_dalek::SigningKey;
// use ed25519_dalek::VerifyingKey;
use peridio_sdk::api::binary_signatures::CreateBinarySignatureParams;
use peridio_sdk::api::binary_signatures::DeleteBinarySignatureParams;
use peridio_sdk::api::Api;
use peridio_sdk::api::ApiOptions;
use snafu::ResultExt;

#[derive(Parser, Debug)]
pub enum BinarySignaturesCommand {
    Create(Command<CreateCommand>),
    Delete(Command<DeleteCommand>),
}

impl BinarySignaturesCommand {
    pub async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        match self {
            Self::Create(cmd) => cmd.run(global_options).await,
            Self::Delete(cmd) => cmd.run(global_options).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct CreateCommand {
    #[arg(
        long,
        short = 'b',
        help = "The PRN of the binary this signature will be associated with."
    )]
    binary_prn: String,
    #[arg(
        long,
        short = 'c',
        conflicts_with = "signature",
        help = "The path of the file to create a signature for.",
        conflicts_with = "signature"
    )]
    binary_content_path: Option<String>,
    #[arg(
        long,
        short = 'g',
        conflicts_with = "signing_key_private",
        conflicts_with = "content_path",
        help = "The hex encoded signature of the SHA256 hash of the binary content."
    )]
    signature: Option<String>,
    #[arg(
        long,
        short = 's',
        conflicts_with = "signing_key_private",
        conflicts_with = "signing_key_public",
        conflicts_with = "signing_key_prn",
        help = "The name of a signing key pair as defined in your Peridio CLI config."
    )]
    signing_key_pair: Option<String>,
    #[arg(
        long,
        conflicts_with = "signature",
        conflicts_with = "signing_key_pair",
        help = "The PEM base64-encoded PKCS #8 private key."
    )]
    signing_key_private: Option<String>,
    #[arg(
        long,
        conflicts_with = "signing_key_public",
        help = "The PRN of the signing key to tell Peridio to verify the signature with."
    )]
    signing_key_prn: String,
    #[arg(
        long,
        conflicts_with = "signing_key_prn",
        help = "The PEM base64-encoded SPKI public key."
    )]
    signing_key_public: Option<String>,
}

impl Command<CreateCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = CreateBinarySignatureParams {
            binary_prn: self.inner.binary_prn,
            signing_key_prn: self.inner.signing_key_prn,
            signature: self.inner.signature.unwrap(),
        };

        // match self.inner.signing_key_pair {
        //     Some(name) => {
        //       pem = File.read()
        //         params.signing_key_private = SigningKey::from_pkcs8_pem(pem.as_str())
        //     }
        //     None => (),
        // };
        // match self.inner.signing_key_pair {
        //     Some(name) => {
        //         params.signing_key_private = VerifyingKey::from_public_key_pem(pem.as_str())
        //     }
        //     None => (),
        // };
        // let hash = "";
        // let signature = private_key.sign(hash);

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .binary_signatures()
            .create(params)
            .await
            .context(ApiSnafu)?
        {
            Some(binary_signature) => print_json!(&binary_signature),
            None => panic!(),
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct DeleteCommand {
    #[arg(long)]
    binary_signature_prn: String,
}

impl Command<DeleteCommand> {
    async fn run(self, global_options: GlobalOptions) -> Result<(), Error> {
        let params = DeleteBinarySignatureParams {
            binary_signature_prn: self.inner.binary_signature_prn,
        };

        let api = Api::new(ApiOptions {
            api_key: global_options.api_key.unwrap(),
            endpoint: global_options.base_url,
            ca_bundle_path: global_options.ca_path,
        });

        match api
            .binary_signatures()
            .delete(params)
            .await
            .context(ApiSnafu)?
        {
            Some(binary_signature) => print_json!(&binary_signature),
            None => panic!(),
        }

        Ok(())
    }
}
