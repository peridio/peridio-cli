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
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::thread::available_parallelism;

pub struct BinaryUploader {
    binary_part_size: u64,
    concurrency: u8,
}

/// Where binary content is read from. Parts are read on demand.
pub enum UploadSource<'a> {
    /// Content on disk; each part is read with its own open + seek.
    File(&'a Path),
    /// Content already held in memory; only the requested range is copied.
    Memory(&'a [u8]),
}

impl UploadSource<'_> {
    pub fn len(&self) -> io::Result<u64> {
        match self {
            UploadSource::File(path) => Ok(std::fs::metadata(path)?.len()),
            UploadSource::Memory(content) => Ok(content.len() as u64),
        }
    }

    /// Reads exactly the bytes in `start..start + len`. Fails with `UnexpectedEof` when the
    /// source ends before `len` bytes are available.
    pub async fn read_part(&self, start: u64, len: u64) -> io::Result<Vec<u8>> {
        let short_read = || {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("expected {len} bytes at offset {start} but the content ended early"),
            )
        };

        match self {
            UploadSource::File(path) => {
                let path = path.to_path_buf();
                let capacity = usize::try_from(len).map_err(io::Error::other)?;
                let buffer = tokio::task::spawn_blocking(move || {
                    // a separate handle per part avoids racing on a shared seek position
                    let mut file = std::fs::File::open(path)?;
                    file.seek(SeekFrom::Start(start))?;
                    let mut buffer = Vec::with_capacity(capacity);
                    file.take(len).read_to_end(&mut buffer)?;
                    Ok::<_, io::Error>(buffer)
                })
                .await
                .map_err(io::Error::other)??;

                if buffer.len() != capacity {
                    return Err(short_read());
                }

                Ok(buffer)
            }
            UploadSource::Memory(content) => {
                let end = start
                    .checked_add(len)
                    .filter(|end| *end <= content.len() as u64)
                    .ok_or_else(short_read)?;

                Ok(content[start as usize..end as usize].to_vec())
            }
        }
    }
}

struct UploadContext<'a> {
    binary: &'a Binary,
    api: &'a Api,
    global_options: GlobalOptions,
    source: UploadSource<'a>,
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

    /// Uploads `source` as binary parts, skipping parts that are already `Valid`.
    ///
    /// Binaries can be multiple GB. Parts are read on demand, so memory stays near
    /// `concurrency * part_size`. Do not buffer or copy the whole source; pass
    /// `UploadSource::File` for on-disk content.
    pub async fn upload(
        &self,
        binary: &Binary,
        api: &Api,
        global_options: GlobalOptions,
        source: UploadSource<'_>,
    ) -> Result<(), Error> {
        let file_size = source.len().map_err(|e| Error::Generic {
            error: format!("Failed to determine binary content size: {e}"),
        })?;

        if let Some(expected_size) = binary.size {
            if expected_size != file_size {
                return Err(Error::Generic {
                    error: format!(
                        "Binary content is {file_size} bytes but the binary expects {expected_size} bytes; the content changed after it was hashed"
                    ),
                });
            }
        }

        let chunks_length = file_size.div_ceil(self.binary_part_size);

        let client = Client::new();
        let binary_parts = self.get_binary_parts(binary, api, &global_options).await?;

        let ctx = UploadContext {
            binary,
            api,
            global_options,
            source,
            file_size,
            chunks_length,
            client: &client,
            binary_parts: &binary_parts,
        };

        self.upload_binary_parts(ctx).await
    }

    async fn upload_binary_parts(&self, ctx: UploadContext<'_>) -> Result<(), Error> {
        eprintln!("Creating binary parts and uploading...");
        let pb = Arc::new(ProgressBar::new(ctx.file_size));
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));

        let ctx = &ctx;

        let results = stream::iter(1..=ctx.chunks_length)
            .map(|index| {
                let pb = Arc::clone(&pb);

                async move {
                    // parts that are already uploaded and valid are skipped
                    if let Some(binary_part) =
                        ctx.binary_parts.iter().find(|x| x.index as u64 == index)
                    {
                        if matches!(binary_part.state, BinaryPartState::Valid) {
                            pb.inc(binary_part.size);
                            return Ok(());
                        }
                    }

                    let start_pos = self.binary_part_size * (index - 1);
                    let part_len = cmp::min(self.binary_part_size, ctx.file_size - start_pos);

                    let chunk_data =
                        ctx.source
                            .read_part(start_pos, part_len)
                            .await
                            .map_err(|e| Error::Generic {
                                error: format!("Failed to read binary part {index}: {e}"),
                            })?;

                    let params = ChunkUploadParams {
                        binary: ctx.binary.clone(),
                        api: ctx.api.clone(),
                        global_options: ctx.global_options.clone(),
                        client: ctx.client.clone(),
                        pb,
                        index,
                        chunk_data,
                    };

                    tokio::spawn(Self::upload_chunk(params))
                        .await
                        .map_err(|e| Error::Generic {
                            error: format!("Upload task for binary part {index} failed: {e}"),
                        })?
                }
            })
            .buffer_unordered(self.concurrency.into())
            .collect::<Vec<Result<(), Error>>>()
            .await;

        pb.finish_and_clear();

        results.into_iter().collect()
    }

    async fn upload_chunk(params: ChunkUploadParams) -> Result<(), Error> {
        let index = params.index;
        let chunk_size = params.chunk_data.len();
        let mut hasher = Sha256::new();
        hasher.update(&params.chunk_data);
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
            .map_err(|e| Error::Generic {
                error: format!(
                    "Failed to create binary part {index}: {e}. Rerun the command to resume the upload."
                ),
            })?
            .ok_or_else(|| Error::Generic {
                error: format!("Cannot create binary part {index}"),
            })?;

        let hash_base64 = general_purpose::STANDARD.encode(hash);

        Self::put_part(
            &params.client,
            bin_part.binary_part.presigned_upload_url,
            params.chunk_data,
            &hash_base64,
            index,
        )
        .await?;

        params.pb.inc(chunk_size as u64);

        Ok(())
    }

    async fn put_part(
        client: &Client,
        url: String,
        chunk_data: Vec<u8>,
        hash_base64: &str,
        index: u64,
    ) -> Result<(), Error> {
        let chunk_size = chunk_data.len();

        let res = client
            .put(url)
            .body(Body::from(chunk_data))
            .header("x-amz-checksum-sha256", hash_base64)
            .header("content-length", chunk_size)
            .header("content-type", "application/octet-stream")
            .send()
            .await
            .map_err(|e| Error::Generic {
                error: format!("Failed to upload binary part {index} to storage: {e}"),
            })?;

        if !(200..=201).contains(&res.status().as_u16()) {
            return Err(Error::Generic {
                error: format!(
                    "Failed to upload binary part {index} to storage (HTTP {})",
                    res.status()
                ),
            });
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sample(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[tokio::test]
    async fn file_read_part_is_bounded_by_part_len() {
        let data = sample(10_000);
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&data).unwrap();

        let source = UploadSource::File(tmp.path());
        assert_eq!(source.len().unwrap(), 10_000);

        // first, middle, and short last part
        for (start, len) in [(0, 4096), (4096, 4096), (8192, 1808)] {
            let part = source.read_part(start, len).await.unwrap();
            assert_eq!(part.len() as u64, len);
            assert_eq!(part, data[start as usize..(start + len) as usize]);
        }
    }

    #[tokio::test]
    async fn short_read_is_an_error() {
        let data = sample(100);
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&data).unwrap();

        for source in [UploadSource::File(tmp.path()), UploadSource::Memory(&data)] {
            let err = source.read_part(50, 100).await.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
            assert!(source.read_part(100, 1).await.is_err());
            assert_eq!(source.read_part(50, 50).await.unwrap(), data[50..]);
        }
    }

    /// Serves one HTTP response with `status` and returns the URL to PUT to.
    async fn mock_storage(status: u16) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/part", listener.local_addr().unwrap());

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];

            // read the headers and the whole body before responding
            loop {
                let n = socket.read(&mut chunk).await.unwrap();
                request.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&request).to_lowercase();
                if let Some(header_end) = text.find("\r\n\r\n") {
                    let body_len = text
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if request.len() >= header_end + 4 + body_len {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }

            let response =
                format!("HTTP/1.1 {status} X\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        url
    }

    #[tokio::test]
    async fn put_part_succeeds_on_2xx() {
        let url = mock_storage(200).await;
        let result =
            BinaryUploader::put_part(&Client::new(), url, vec![1, 2, 3], "aGFzaA==", 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn put_part_reports_the_part_and_status_on_failure() {
        let url = mock_storage(500).await;
        let err = BinaryUploader::put_part(&Client::new(), url, vec![1, 2, 3], "aGFzaA==", 7)
            .await
            .unwrap_err();
        let message = err.to_string();
        assert!(message.contains("part 7"), "{message}");
        assert!(message.contains("500"), "{message}");
    }

    #[tokio::test]
    async fn put_part_reports_connection_failures() {
        // nothing listens on port 1
        let result = BinaryUploader::put_part(
            &Client::new(),
            "http://127.0.0.1:1/part".to_string(),
            vec![1],
            "aGFzaA==",
            3,
        )
        .await;
        assert!(result.unwrap_err().to_string().contains("part 3"));
    }

    #[tokio::test]
    async fn memory_and_file_sources_agree() {
        let data = sample(5_000);
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&data).unwrap();

        let file = UploadSource::File(tmp.path());
        let memory = UploadSource::Memory(&data);
        assert_eq!(file.len().unwrap(), memory.len().unwrap());

        for start in [0u64, 1024, 3976] {
            assert_eq!(
                file.read_part(start, 1024).await.unwrap(),
                memory.read_part(start, 1024).await.unwrap()
            );
        }
    }

    #[tokio::test]
    async fn missing_file_is_an_error() {
        let source = UploadSource::File(Path::new("/nonexistent/peridio-test-file"));
        assert!(source.len().is_err());
        assert!(source.read_part(0, 10).await.is_err());
    }
}
