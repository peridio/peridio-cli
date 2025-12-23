use crate::api::binary_parts;
use crate::Error;
use crate::GlobalOptions;
use base64::engine::general_purpose;
use base64::Engine;
use futures_util::stream;
use futures_util::StreamExt;
use indicatif::ProgressBar;
use indicatif::ProgressState;
use indicatif::ProgressStyle;
use peridio_sdk::api::binaries::Binary;
use peridio_sdk::api::binary_parts::{BinaryPartState, ListBinaryPart};
use peridio_sdk::api::Api;
use reqwest::Body;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::cmp;
use std::io;
use std::sync::Arc;
use std::thread::available_parallelism;

pub struct BinaryUploader {
    binary_part_size: u64,
    concurrency: u8,
}

struct UploadContext<'a> {
    binary: &'a Binary,
    api: &'a Api,
    global_options: GlobalOptions,
    content: &'a [u8],
    file_size: u64,
    chunks_length: u64,
    client: &'a Client,
    binary_parts: &'a [ListBinaryPart],
}

struct ChunkUploadParams {
    binary: Binary,
    api: Api,
    global_options: GlobalOptions,
    client: Client,
    pb: Arc<ProgressBar>,
    index: u64,
    chunk_data: Vec<u8>,
}

impl Default for BinaryUploader {
    fn default() -> Self {
        Self {
            binary_part_size: 5242880, // 5MB default
            concurrency: cmp::min(available_parallelism().unwrap().get() * 2, 16)
                .try_into()
                .unwrap(),
        }
    }
}

impl BinaryUploader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_part_size(mut self, size: u64) -> Self {
        self.binary_part_size = size;
        self
    }

    pub fn with_concurrency(mut self, concurrency: u8) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Upload binary content from memory
    pub async fn upload_from_memory(
        &self,
        binary: &Binary,
        api: &Api,
        global_options: GlobalOptions,
        content: &[u8],
    ) -> Result<(), Error> {
        let file_size = content.len() as u64;
        let chunks_length = file_size.div_ceil(self.binary_part_size);

        let client = Client::new();
        let binary_parts = self.get_binary_parts(binary, api, &global_options).await?;

        let ctx = UploadContext {
            binary,
            api,
            global_options,
            content,
            file_size,
            chunks_length,
            client: &client,
            binary_parts: &binary_parts,
        };

        self.upload_binary_parts_from_memory(ctx).await
    }

    async fn upload_binary_parts_from_memory(&self, ctx: UploadContext<'_>) -> Result<(), Error> {
        eprintln!("Creating binary parts and uploading...");
        let pb = Arc::new(ProgressBar::new(ctx.file_size));
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));

        // Convert content to Arc for sharing across threads
        let content = Arc::new(ctx.content.to_vec());

        let result = stream::iter(1..=ctx.chunks_length)
            .map(|index| {
                let binary_part_size = self.binary_part_size;
                let content = Arc::clone(&content);
                let binary_parts = ctx.binary_parts.to_vec();
                let pb = Arc::clone(&pb);

                let params = ChunkUploadParams {
                    binary: ctx.binary.clone(),
                    api: ctx.api.clone(),
                    global_options: ctx.global_options.clone(),
                    client: ctx.client.clone(),
                    pb: Arc::clone(&pb),
                    index,
                    chunk_data: Vec::new(), // Will be filled below
                };

                tokio::spawn(async move {
                    // we ignore the ones we already created
                    if let Some(binary_part) = binary_parts.iter().find(|x| x.index as u64 == index)
                    {
                        if matches!(binary_part.state, BinaryPartState::Valid) {
                            pb.inc(binary_part.size);
                            return;
                        }
                    }

                    let start_pos = (binary_part_size * (index - 1)) as usize;
                    let end_pos =
                        std::cmp::min(start_pos + binary_part_size as usize, content.len());

                    if start_pos < content.len() {
                        let chunk = content[start_pos..end_pos].to_vec();
                        let mut params = params;
                        params.chunk_data = chunk.clone();

                        Self::upload_chunk(params).await;
                    }
                })
            })
            .buffer_unordered(self.concurrency.into());

        let _ = result.collect::<Vec<_>>().await;
        pb.finish_and_clear();

        Ok(())
    }

    async fn upload_chunk(params: ChunkUploadParams) {
        let chunk_size = params.chunk_data.len();
        let mut hasher = Sha256::new();
        let _ = io::copy(&mut &params.chunk_data[..], &mut hasher).unwrap();
        let hash = hasher.finalize();

        // push those bytes to the server
        let create_command = binary_parts::CreateCommand {
            binary_prn: params.binary.prn.clone(),
            expected_binary_size: params.binary.size,
            index: params.index as u16,
            hash: format!("{hash:x}"),
            api: Some(params.api.clone()),
            size: chunk_size as u64,
            binary_content_path: None,
        };

        let bin_part = create_command
            .run(params.global_options)
            .await
            .expect("Error while creating a binary part binary part")
            .expect("Cannot create a binary part");

        // do amazon request
        let body = Body::from(params.chunk_data);

        let hash_base64 = general_purpose::STANDARD.encode(hash);

        let res = params
            .client
            .put(bin_part.binary_part.presigned_upload_url)
            .body(body)
            .header("x-amz-checksum-sha256", &hash_base64)
            .header("content-length", chunk_size)
            .header("content-type", "application/octet-stream")
            .send()
            .await
            .unwrap();

        params.pb.inc(chunk_size.try_into().unwrap());

        if !(200..=201).contains(&res.status().as_u16()) {
            panic!("Wasn't able to upload binary to amazon S3")
        };
    }

    async fn get_binary_parts(
        &self,
        binary: &Binary,
        api: &Api,
        global_options: &GlobalOptions,
    ) -> Result<Vec<ListBinaryPart>, Error> {
        let list_command = binary_parts::ListCommand {
            binary_prn: binary.prn.clone(),
            api: Some(api.clone()),
        };

        let binary_parts = match list_command.run(global_options.clone()).await? {
            Some(binary_parts) => binary_parts.binary_parts,
            None => Vec::new(),
        };

        Ok(binary_parts)
    }
}
