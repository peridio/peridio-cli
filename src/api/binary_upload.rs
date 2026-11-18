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
use std::future::Future;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::available_parallelism;
use std::time::{Duration, Instant};

/// The API accepts part indexes 1 through this value.
const MAX_PARTS: u64 = 10_000;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// A part upload is abandoned when no part data moves for this long, however slow the link is.
const STALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Part data is handed to the HTTP client in pieces of this size, so progress is observable.
const BODY_PIECE_SIZE: usize = 64 * 1024;

/// Number of parts needed for `file_size`, or an error when it exceeds the API's part limit.
fn part_count(file_size: u64, part_size: u64) -> Result<u64, Error> {
    let parts = file_size.div_ceil(part_size);

    if parts > MAX_PARTS {
        return Err(Error::Generic {
            error: format!(
                "Binary content of {file_size} bytes needs {parts} parts at a part size of {part_size} bytes, but at most {MAX_PARTS} are allowed; pass --binary-part-size {} or larger",
                file_size.div_ceil(MAX_PARTS)
            ),
        });
    }

    Ok(parts)
}

/// After the last piece is handed over, the client still has to flush its buffers and wait for the
/// response, so the stall window is this many times longer.
const TAIL_STALL_FACTOR: u32 = 4;

/// How far a part's body has been handed to the HTTP client. The client asks for the next piece
/// only as its buffers drain, so the piece count follows the connection.
#[derive(Default)]
struct BodyProgress {
    pieces: AtomicU64,
    all_handed_over: AtomicBool,
}

/// The request error and its causes as one line. The presigned URL carries credentials, so the
/// URL is dropped from the error and scrubbed from the text of every cause.
fn describe_request_error(error: reqwest::Error, url: &str) -> String {
    let mut causes = Vec::new();
    let mut source = std::error::Error::source(&error);

    while let Some(cause) = source {
        causes.push(cause.to_string());
        source = cause.source();
    }

    let mut text = error.without_url().to_string();

    for cause in causes {
        text.push_str(": ");
        text.push_str(&cause);
    }

    redact_url(&text, url)
}

/// Replaces `url`, and its query string wherever it appears alone, in `text`. The percent-decoded
/// form of the URL is scrubbed as well, since a cause may print it unescaped.
fn redact_url(text: &str, url: &str) -> String {
    let decoded = percent_decode(url);
    let candidates = [url, decoded.as_str()];
    let mut redacted = text.to_string();

    // whole URLs first, so a URL is not broken up by the query replacement below
    for candidate in candidates {
        redacted = redacted.replace(candidate, "<url>");
    }

    for candidate in candidates {
        if let Some((_, query)) = candidate.split_once('?') {
            if !query.is_empty() {
                redacted = redacted.replace(query, "<redacted>");
            }
        }
    }

    redacted
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        let escaped = (bytes[index] == b'%'
            && index + 2 < bytes.len()
            && bytes[index + 1].is_ascii_hexdigit()
            && bytes[index + 2].is_ascii_hexdigit())
        .then(|| std::str::from_utf8(&bytes[index + 1..index + 3]).ok())
        .flatten()
        .and_then(|hex| u8::from_str_radix(hex, 16).ok());

        match escaped {
            Some(byte) => {
                decoded.push(byte);
                index += 3;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

/// Awaits `future`, giving up with how long nothing moved once `progress` has not changed for
/// `stall` (`stall * TAIL_STALL_FACTOR` once the whole body has been handed over).
async fn until_stalled<T>(
    future: impl Future<Output = T>,
    progress: &BodyProgress,
    stall: Duration,
) -> Result<T, Duration> {
    tokio::pin!(future);

    let tick = cmp::max(stall / 4, Duration::from_millis(1));
    let mut last_seen = progress.pieces.load(Ordering::Relaxed);
    let mut last_change = Instant::now();

    loop {
        match tokio::time::timeout(tick, &mut future).await {
            Ok(output) => return Ok(output),
            Err(_) => {
                let current = progress.pieces.load(Ordering::Relaxed);

                if current != last_seen {
                    last_seen = current;
                    last_change = Instant::now();
                    continue;
                }

                let allowed = if progress.all_handed_over.load(Ordering::Relaxed) {
                    stall * TAIL_STALL_FACTOR
                } else {
                    stall
                };

                if last_change.elapsed() >= allowed {
                    return Err(last_change.elapsed());
                }
            }
        }
    }
}

/// Runs `upload_part` for parts `1..=total_parts`, at most `concurrency` at a time. Every part is
/// attempted even after a failure.
async fn run_parts<F, Fut>(
    total_parts: u64,
    concurrency: usize,
    upload_part: F,
) -> Result<(), Error>
where
    F: Fn(u64) -> Fut,
    Fut: Future<Output = Result<(), Error>>,
{
    let failures = stream::iter(1..=total_parts)
        .map(|index| {
            let part = upload_part(index);
            async move { (index, part.await) }
        })
        .buffer_unordered(concurrency)
        .filter_map(|(index, result)| async move { result.err().map(|error| (index, error)) })
        .collect::<Vec<_>>()
        .await;

    summarize_failures(total_parts, failures)
}

/// Reports every failed part, so a user can tell one failure from many.
fn summarize_failures(total_parts: u64, mut failures: Vec<(u64, Error)>) -> Result<(), Error> {
    if failures.is_empty() {
        return Ok(());
    }

    const LISTED: usize = 20;

    failures.sort_by_key(|(index, _)| *index);

    let mut listed = failures
        .iter()
        .take(LISTED)
        .map(|(index, _)| index.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    if failures.len() > LISTED {
        listed.push_str(&format!(", and {} more", failures.len() - LISTED));
    }

    Err(Error::Generic {
        error: format!(
            "{} of {total_parts} binary parts failed to upload (parts: {listed}). Part {} failed: {}. Rerun the command to resume the upload.",
            failures.len(),
            failures[0].0,
            failures[0].1
        ),
    })
}

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

/// A part to register with the API before its data is sent to storage.
struct NewPart {
    index: u64,
    /// Lowercase hex SHA-256 of the part's data.
    hash: String,
    size: u64,
}

/// What `upload_part` needs to read and send one part, independent of the Peridio API.
struct PartUploadPlan<'a> {
    source: &'a UploadSource<'a>,
    binary_parts: &'a [ListBinaryPart],
    part_size: u64,
    file_size: u64,
    client: &'a Client,
}

/// Uploads part `index`, unless it is already valid. `create_part` registers the part with the
/// API and returns the presigned URL to send its data to.
async fn upload_part<C, Fut>(
    plan: &PartUploadPlan<'_>,
    pb: Arc<ProgressBar>,
    index: u64,
    create_part: C,
) -> Result<(), Error>
where
    C: FnOnce(NewPart) -> Fut + Send + 'static,
    Fut: Future<Output = Result<Option<String>, Error>> + Send + 'static,
{
    // parts that are already uploaded and valid are skipped
    if let Some(binary_part) = plan.binary_parts.iter().find(|x| x.index as u64 == index) {
        if matches!(binary_part.state, BinaryPartState::Valid) {
            pb.inc(binary_part.size);
            return Ok(());
        }
    }

    let start_pos = plan.part_size * (index - 1);
    let part_len = cmp::min(plan.part_size, plan.file_size - start_pos);

    let chunk_data = plan
        .source
        .read_part(start_pos, part_len)
        .await
        .map_err(|e| Error::Generic {
            error: format!("Failed to read binary part {index}: {e}"),
        })?;

    tokio::spawn(upload_chunk(
        plan.client.clone(),
        pb,
        index,
        chunk_data,
        create_part,
    ))
    .await
    .map_err(|e| Error::Generic {
        error: format!("Upload task for binary part {index} failed: {e}"),
    })?
}

async fn upload_chunk<C, Fut>(
    client: Client,
    pb: Arc<ProgressBar>,
    index: u64,
    chunk_data: Vec<u8>,
    create_part: C,
) -> Result<(), Error>
where
    C: FnOnce(NewPart) -> Fut,
    Fut: Future<Output = Result<Option<String>, Error>>,
{
    let chunk_size = chunk_data.len();
    let mut hasher = Sha256::new();
    hasher.update(&chunk_data);
    let hash = hasher.finalize();

    let url = create_part(NewPart {
        index,
        hash: format!("{hash:x}"),
        size: chunk_size as u64,
    })
    .await
    .map_err(|e| Error::Generic {
        error: format!("Failed to create binary part {index}: {e}"),
    })?
    .ok_or_else(|| Error::Generic {
        error: format!("Cannot create binary part {index}"),
    })?;

    let hash_base64 = general_purpose::STANDARD.encode(hash);

    BinaryUploader::put_part(&client, url, chunk_data, &hash_base64, index, STALL_TIMEOUT).await?;

    pb.inc(chunk_size as u64);

    Ok(())
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
                        "Binary content is {file_size} bytes but {expected_size} bytes were expected; the content does not match the size recorded for the binary"
                    ),
                });
            }
        }

        let chunks_length = part_count(file_size, self.binary_part_size)?;

        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|e| Error::Generic {
                error: format!("Failed to create HTTP client: {e}"),
            })?;
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

        let plan = PartUploadPlan {
            source: &ctx.source,
            binary_parts: ctx.binary_parts,
            part_size: self.binary_part_size,
            file_size: ctx.file_size,
            client: ctx.client,
        };
        let plan = &plan;
        let ctx = &ctx;

        let result = run_parts(ctx.chunks_length, self.concurrency.into(), |index| {
            let binary = ctx.binary.clone();
            let api = ctx.api.clone();
            let global_options = ctx.global_options.clone();
            let content_size = ctx.file_size;

            upload_part(
                plan,
                Arc::clone(&pb),
                index,
                move |part: NewPart| async move {
                    let command = binary_parts::CreateCommand {
                        binary_prn: binary.prn,
                        expected_binary_size: Some(content_size),
                        index: part.index as u16,
                        hash: part.hash,
                        api: Some(api),
                        size: part.size,
                        binary_content_path: None,
                    };

                    Ok(command
                        .run(global_options)
                        .await?
                        .map(|created| created.binary_part.presigned_upload_url))
                },
            )
        })
        .await;

        pb.finish_and_clear();

        result
    }

    async fn put_part(
        client: &Client,
        url: String,
        chunk_data: Vec<u8>,
        hash_base64: &str,
        index: u64,
        stall: Duration,
    ) -> Result<(), Error> {
        let chunk_size = chunk_data.len();

        let progress = Arc::new(BodyProgress::default());
        let body = stream::unfold(
            (chunk_data, 0usize, Arc::clone(&progress)),
            |(data, position, progress)| async move {
                if position >= data.len() {
                    progress.all_handed_over.store(true, Ordering::Relaxed);
                    return None;
                }

                let end = cmp::min(position + BODY_PIECE_SIZE, data.len());
                let piece = data[position..end].to_vec();
                progress.pieces.fetch_add(1, Ordering::Relaxed);

                // With a declared `content-length` the client may not poll again after the
                // last piece, so the flag is set when that piece is handed over
                if end == data.len() {
                    progress.all_handed_over.store(true, Ordering::Relaxed);
                }

                Some((Ok::<_, io::Error>(piece), (data, end, progress)))
            },
        );

        let request = client
            .put(&url)
            .body(Body::wrap_stream(body))
            .header("x-amz-checksum-sha256", hash_base64)
            .header("content-length", chunk_size)
            .header("content-type", "application/octet-stream")
            .send();

        let res = until_stalled(request, &progress, stall)
            .await
            .map_err(|idle| Error::Generic {
                error: format!(
                    "Failed to upload binary part {index} to storage: no data moved for {} seconds. If this keeps happening on a slow connection, lower --concurrency",
                    idle.as_secs()
                ),
            })?
            .map_err(|e| {
                let reason = if e.is_timeout() {
                    "timed out"
                } else if e.is_connect() {
                    "could not connect"
                } else {
                    "request failed"
                };

                Error::Generic {
                    error: format!(
                        "Failed to upload binary part {index} to storage: {reason} ({})",
                        describe_request_error(e, &url)
                    ),
                }
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

    #[test]
    fn part_count_enforces_the_api_limit() {
        assert_eq!(part_count(0, 5 << 20).unwrap(), 0);
        assert_eq!(part_count(10_000 * 5, 5).unwrap(), 10_000);

        let err = part_count(10_000 * 5 + 1, 5).unwrap_err().to_string();
        assert!(err.contains("10001 parts"), "{err}");
        assert!(err.contains("--binary-part-size 6"), "{err}");

        // the default part size tops out just under 49 GiB
        assert!(part_count(49 << 30, 5 << 20).is_err());
        assert!(part_count(48 << 30, 5 << 20).is_ok());
    }

    #[test]
    fn summarize_failures_lists_failing_parts() {
        let failure = |i: u64| {
            (
                i,
                Error::Generic {
                    error: format!("boom {i}"),
                },
            )
        };

        assert!(summarize_failures(5, vec![]).is_ok());

        let err = summarize_failures(5, vec![failure(4), failure(2)])
            .unwrap_err()
            .to_string();
        assert!(err.starts_with("2 of 5 binary parts failed"), "{err}");
        assert!(err.contains("parts: 2, 4"), "{err}");
        assert!(err.contains("Part 2 failed: boom 2"), "{err}");

        let many = (1..=25).map(failure).collect();
        let err = summarize_failures(100, many).unwrap_err().to_string();
        assert!(err.starts_with("25 of 100 binary parts failed"), "{err}");
        assert!(err.contains("19, 20, and 5 more"), "{err}");
        assert!(!err.contains("21,"), "{err}");
    }

    #[tokio::test]
    async fn run_parts_attempts_every_part_and_reports_failures() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempted = AtomicUsize::new(0);
        let in_flight = AtomicUsize::new(0);
        let max_in_flight = AtomicUsize::new(0);

        let result = run_parts(8, 3, |index| {
            let (attempted, in_flight, max_in_flight) = (&attempted, &in_flight, &max_in_flight);
            async move {
                attempted.fetch_add(1, Ordering::SeqCst);
                let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                max_in_flight.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(5)).await;
                in_flight.fetch_sub(1, Ordering::SeqCst);

                if index == 2 || index == 5 {
                    Err(Error::Generic {
                        error: format!("boom {index}"),
                    })
                } else {
                    Ok(())
                }
            }
        })
        .await;

        let err = result.unwrap_err().to_string();
        assert!(err.starts_with("2 of 8 binary parts failed"), "{err}");
        assert!(err.contains("parts: 2, 5"), "{err}");
        assert_eq!(attempted.load(Ordering::SeqCst), 8);
        assert!(max_in_flight.load(Ordering::SeqCst) <= 3);
    }

    #[tokio::test]
    async fn run_parts_succeeds_when_every_part_succeeds() {
        assert!(run_parts(4, 2, |_| async { Ok(()) }).await.is_ok());
    }

    #[tokio::test]
    async fn until_stalled_gives_up_when_progress_stops() {
        let progress = BodyProgress::default();
        let idle = until_stalled(
            std::future::pending::<()>(),
            &progress,
            Duration::from_millis(100),
        )
        .await
        .unwrap_err();

        // measured from the last change, so never less than the window
        assert!(idle >= Duration::from_millis(100), "{idle:?}");
    }

    #[tokio::test]
    async fn until_stalled_waits_while_progress_continues() {
        let progress = BodyProgress::default();

        // takes about 400ms, past the 300ms stall window, but progresses every 20ms
        let slow = async {
            for _ in 0..20 {
                tokio::time::sleep(Duration::from_millis(20)).await;
                progress.pieces.fetch_add(1, Ordering::Relaxed);
            }
            "done"
        };

        let result = until_stalled(slow, &progress, Duration::from_millis(300)).await;
        assert_eq!(result, Ok("done"));
    }

    #[tokio::test]
    async fn until_stalled_allows_a_longer_tail_once_the_body_is_handed_over() {
        let progress = BodyProgress::default();
        progress.all_handed_over.store(true, Ordering::Relaxed);

        // 250ms of silence is past the 100ms window but inside the 400ms tail window
        let flushing = async {
            tokio::time::sleep(Duration::from_millis(250)).await;
            "done"
        };
        let result = until_stalled(flushing, &progress, Duration::from_millis(100)).await;
        assert_eq!(result, Ok("done"));

        // a genuinely stuck tail is still abandoned, after the longer window
        let idle = until_stalled(
            std::future::pending::<()>(),
            &progress,
            Duration::from_millis(100),
        )
        .await
        .unwrap_err();
        assert!(idle >= Duration::from_millis(400), "{idle:?}");
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

    /// Accepts one request and answers it with `status`, or never answers when `None`. Returns
    /// the URL to PUT to and a handle that yields the raw request once it has been read.
    async fn mock_storage(status: Option<u16>) -> (String, tokio::task::JoinHandle<Vec<u8>>) {
        mock_storage_after(status, Duration::ZERO).await
    }

    /// Like `mock_storage`, but waits `delay` after reading the whole request before answering.
    async fn mock_storage_after(
        status: Option<u16>,
        delay: Duration,
    ) -> (String, tokio::task::JoinHandle<Vec<u8>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/part", listener.local_addr().unwrap());

        let server = tokio::spawn(async move {
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

            match status {
                Some(status) => {
                    tokio::time::sleep(delay).await;
                    let response = format!(
                        "HTTP/1.1 {status} X\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                    );
                    socket.write_all(response.as_bytes()).await.unwrap();
                }
                None => tokio::time::sleep(Duration::from_secs(30)).await,
            }

            request
        });

        (url, server)
    }

    const TEST_STALL: Duration = Duration::from_secs(10);

    fn listed(index: u16, state: BinaryPartState, size: u64) -> ListBinaryPart {
        ListBinaryPart {
            binary_prn: None,
            hash: "hash".to_string(),
            index,
            inserted_at: String::new(),
            organization_prn: String::new(),
            prn: String::new(),
            size,
            state,
            updated_at: String::new(),
        }
    }

    fn hidden_progress() -> Arc<ProgressBar> {
        Arc::new(ProgressBar::hidden())
    }

    #[test]
    fn redact_url_scrubs_the_url_and_its_query() {
        let url = "https://bucket.example/obj?X-Amz-Signature=secret&partNumber=1";

        assert_eq!(
            redact_url(&format!("failed for {url}"), url),
            "failed for <url>"
        );
        assert_eq!(
            redact_url("bad query X-Amz-Signature=secret&partNumber=1 here", url),
            "bad query <redacted> here"
        );
        assert_eq!(redact_url("no secrets", "https://host/obj"), "no secrets");
    }

    #[test]
    fn redact_url_scrubs_percent_encoded_and_decoded_forms() {
        let url = "https://bucket.example/obj?X-Amz-Credential=AKIA%2F20260101%2Fus-east-1&X-Amz-Signature=secret";

        // the URL as given
        assert_eq!(
            redact_url(&format!("failed for {url}"), url),
            "failed for <url>"
        );

        // a cause that prints the URL with its escapes resolved
        let decoded = "https://bucket.example/obj?X-Amz-Credential=AKIA/20260101/us-east-1&X-Amz-Signature=secret";
        let message = redact_url(&format!("failed for {decoded}"), url);
        assert_eq!(message, "failed for <url>");

        // only the decoded query string
        let message = redact_url(
            "bad X-Amz-Credential=AKIA/20260101/us-east-1&X-Amz-Signature=secret here",
            url,
        );
        assert_eq!(message, "bad <redacted> here");
        assert!(!message.contains("secret"));
    }

    #[test]
    fn redact_url_replaces_a_decoded_url_whole_when_only_the_path_has_escapes() {
        let url = "https://h/a%2Fb?sig=secret";

        // the decoded URL must be matched before its query is scrubbed on its own
        let message = redact_url("failed for https://h/a/b?sig=secret", url);
        assert_eq!(message, "failed for <url>");

        let message = redact_url("failed for https://h/a%2Fb?sig=secret", url);
        assert_eq!(message, "failed for <url>");
    }

    #[test]
    fn percent_decode_resolves_valid_escapes_only() {
        assert_eq!(percent_decode("a%2Fb%20c"), "a/b c");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("bad%zzescape%2"), "bad%zzescape%2");
        // from_str_radix would accept a sign, but an escape needs two hex digits
        assert_eq!(percent_decode("%+f"), "%+f");
        assert_eq!(percent_decode("%-1"), "%-1");
        assert_eq!(percent_decode("plain"), "plain");
    }

    #[tokio::test]
    async fn upload_chunk_registers_then_sends_the_part_and_counts_it() {
        use sha2::Digest;

        let data = sample(2_000);
        let expected_hash = format!("{:x}", Sha256::digest(&data));
        let (url, server) = mock_storage(Some(200)).await;
        let pb = hidden_progress();

        let result = upload_chunk(Client::new(), Arc::clone(&pb), 3, data.clone(), {
            let expected_hash = expected_hash.clone();
            move |part: NewPart| async move {
                assert_eq!(part.index, 3);
                assert_eq!(part.size, 2_000);
                assert_eq!(part.hash, expected_hash);
                Ok(Some(url))
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(pb.position(), 2_000);

        let request = server.await.unwrap();
        let header_end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        assert_eq!(&request[header_end..], &data[..]);
    }

    #[tokio::test]
    async fn upload_chunk_reports_a_failed_create_without_sending_or_counting() {
        let pb = hidden_progress();

        let err = upload_chunk(
            Client::new(),
            Arc::clone(&pb),
            3,
            vec![1, 2, 3],
            |_| async {
                Err(Error::Generic {
                    error: "denied".to_string(),
                })
            },
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(err.contains("Failed to create binary part 3"), "{err}");
        assert!(err.contains("denied"), "{err}");
        assert_eq!(pb.position(), 0);

        let err = upload_chunk(Client::new(), Arc::clone(&pb), 4, vec![1], |_| async {
            Ok(None)
        })
        .await
        .unwrap_err()
        .to_string();

        assert!(err.contains("Cannot create binary part 4"), "{err}");
        assert_eq!(pb.position(), 0);
    }

    #[tokio::test]
    async fn upload_chunk_fails_and_does_not_count_a_rejected_put() {
        let (url, _server) = mock_storage(Some(500)).await;
        let pb = hidden_progress();

        let err = upload_chunk(
            Client::new(),
            Arc::clone(&pb),
            5,
            vec![1, 2, 3],
            |_| async { Ok(Some(url)) },
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(err.contains("part 5") && err.contains("500"), "{err}");
        assert_eq!(pb.position(), 0);
    }

    #[tokio::test]
    async fn upload_part_skips_a_valid_part_without_reading_it() {
        // the file does not exist, so any read of the source would fail the upload
        let source = UploadSource::File(Path::new("/nonexistent/peridio-test-file"));
        let parts = [listed(1, BinaryPartState::Valid, 4)];
        let client = Client::new();
        let plan = PartUploadPlan {
            source: &source,
            binary_parts: &parts,
            part_size: 4,
            file_size: 10,
            client: &client,
        };
        let pb = hidden_progress();

        let result = upload_part(&plan, Arc::clone(&pb), 1, |_| async {
            Err(Error::Generic {
                error: "must not be called".to_string(),
            })
        })
        .await;

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(pb.position(), 4);

        // a part that is not valid does read the source, and fails because it is missing
        let err = upload_part(&plan, Arc::clone(&pb), 2, |_| async {
            Ok(Some("http://127.0.0.1:1/unused".to_string()))
        })
        .await
        .unwrap_err()
        .to_string();
        assert!(err.contains("Failed to read binary part 2"), "{err}");
    }

    #[tokio::test]
    async fn upload_part_skips_a_valid_part_without_creating_it() {
        let data = sample(10);
        let source = UploadSource::Memory(&data);
        let parts = [listed(1, BinaryPartState::Valid, 4)];
        let client = Client::new();
        let plan = PartUploadPlan {
            source: &source,
            binary_parts: &parts,
            part_size: 4,
            file_size: 10,
            client: &client,
        };
        let pb = hidden_progress();

        let result = upload_part(&plan, Arc::clone(&pb), 1, |_| async {
            Err(Error::Generic {
                error: "must not be called".to_string(),
            })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(pb.position(), 4);
    }

    #[tokio::test]
    async fn upload_part_sends_only_its_own_range() {
        let data = sample(10);
        let source = UploadSource::Memory(&data);
        // part 3 is not valid; part 1 being valid must not affect it
        let parts = [
            listed(1, BinaryPartState::Valid, 4),
            listed(3, BinaryPartState::InvalidHash, 2),
        ];
        let client = Client::new();
        let plan = PartUploadPlan {
            source: &source,
            binary_parts: &parts,
            part_size: 4,
            file_size: 10,
            client: &client,
        };
        let (url, server) = mock_storage(Some(200)).await;
        let pb = hidden_progress();

        let result = upload_part(&plan, Arc::clone(&pb), 3, move |part: NewPart| async move {
            assert_eq!(part.size, 2);
            Ok(Some(url))
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(pb.position(), 2);

        let request = server.await.unwrap();
        let header_end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        assert_eq!(&request[header_end..], &data[8..10]);
    }

    #[tokio::test]
    async fn upload_part_reports_an_unreadable_part() {
        let source = UploadSource::File(Path::new("/nonexistent/peridio-test-file"));
        let client = Client::new();
        let plan = PartUploadPlan {
            source: &source,
            binary_parts: &[],
            part_size: 4,
            file_size: 4,
            client: &client,
        };

        let err = upload_part(&plan, hidden_progress(), 1, |_| async {
            Ok(Some("http://127.0.0.1:1/unused".to_string()))
        })
        .await
        .unwrap_err()
        .to_string();

        assert!(err.contains("Failed to read binary part 1"), "{err}");
    }

    #[tokio::test]
    async fn put_part_succeeds_on_2xx() {
        let (url, _server) = mock_storage(Some(200)).await;
        let result = BinaryUploader::put_part(
            &Client::new(),
            url,
            vec![1, 2, 3],
            "aGFzaA==",
            1,
            TEST_STALL,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn put_part_sends_the_whole_part_with_its_length() {
        // spans several body pieces
        let data = sample(3 * BODY_PIECE_SIZE + 123);
        let (url, server) = mock_storage(Some(200)).await;

        let result = tokio::time::timeout(
            Duration::from_secs(10),
            BinaryUploader::put_part(&Client::new(), url, data.clone(), "aGFzaA==", 1, TEST_STALL),
        )
        .await
        .expect("upload did not finish");
        assert!(result.is_ok());

        let request = server.await.unwrap();
        let header_end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        let headers = String::from_utf8_lossy(&request[..header_end]).to_lowercase();

        assert!(
            headers.contains(&format!("content-length: {}", data.len())),
            "{headers}"
        );
        assert!(
            headers.contains(&format!(
                "x-amz-checksum-sha256: {}",
                "aGFzaA==".to_lowercase()
            )),
            "{headers}"
        );
        assert_eq!(&request[header_end..], &data[..]);
    }

    #[tokio::test]
    async fn put_part_reports_the_part_and_status_on_failure() {
        let (url, _server) = mock_storage(Some(500)).await;
        let err = BinaryUploader::put_part(
            &Client::new(),
            url,
            vec![1, 2, 3],
            "aGFzaA==",
            7,
            TEST_STALL,
        )
        .await
        .unwrap_err();
        let message = err.to_string();
        assert!(message.contains("part 7"), "{message}");
        assert!(message.contains("500"), "{message}");
    }

    #[tokio::test]
    async fn put_part_allows_a_slow_response_after_the_whole_part_is_sent() {
        // several pieces, so the last piece is not the first
        let data = sample(2 * BODY_PIECE_SIZE + 5);

        // storage answers 250ms after the whole request arrived: past the 100ms stall window,
        // inside the 400ms tail window that applies once the whole part has been handed over
        let (url, _server) = mock_storage_after(Some(200), Duration::from_millis(250)).await;
        let result = BinaryUploader::put_part(
            &Client::new(),
            url,
            data.clone(),
            "aGFzaA==",
            1,
            Duration::from_millis(100),
        )
        .await;
        assert!(result.is_ok(), "{result:?}");

        // a response slower than the tail window is still abandoned
        let (url, _server) = mock_storage_after(Some(200), Duration::from_millis(900)).await;
        let err = BinaryUploader::put_part(
            &Client::new(),
            url,
            data,
            "aGFzaA==",
            2,
            Duration::from_millis(100),
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("part 2") && err.contains("no data moved"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn put_part_gives_up_when_storage_stops_responding() {
        let (url, _server) = mock_storage(None).await;
        let url = format!("{url}?X-Amz-Signature=secret");

        let err = BinaryUploader::put_part(
            &Client::new(),
            url,
            vec![1, 2, 3],
            "aGFzaA==",
            4,
            Duration::from_millis(200),
        )
        .await
        .unwrap_err();
        let message = err.to_string();
        assert!(message.contains("part 4"), "{message}");
        assert!(message.contains("no data moved"), "{message}");
        assert!(message.contains("--concurrency"), "{message}");
        assert!(!message.contains("secret"), "{message}");
    }

    #[tokio::test]
    async fn put_part_reports_connection_failures() {
        // nothing listens on port 1
        let result = BinaryUploader::put_part(
            &Client::new(),
            "http://127.0.0.1:1/part?X-Amz-Signature=secret".to_string(),
            vec![1],
            "aGFzaA==",
            3,
            TEST_STALL,
        )
        .await;
        let message = result.unwrap_err().to_string();
        assert!(message.contains("part 3"), "{message}");
        assert!(message.contains("could not connect"), "{message}");
        assert!(message.to_lowercase().contains("refused"), "{message}");
        assert!(!message.contains("secret"), "{message}");
        assert!(!message.contains("127.0.0.1"), "{message}");
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
