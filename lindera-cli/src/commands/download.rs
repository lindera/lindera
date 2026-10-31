use std::fs::{self, File};
use std::io::{self, BufWriter, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera_cli::get_version;

use super::io_err;
use crate::dictionary_registry::{
    DOWNLOADABLE_DICTIONARIES, dictionaries_dir, dictionary_dir, download_url,
    installed_lindera_version, is_complete_dictionary_dir, missing_dictionary_files,
};

/// Buffer size for streaming the downloaded archive to disk.
const CHUNK_SIZE: usize = 64 * 1024;

/// Connect timeout for the download request.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Response data returned by a [`Fetcher`].
pub(crate) struct FetchResponse {
    /// HTTP status code of the response.
    pub(crate) status: u16,
    /// Value of the `Content-Length` header, when known.
    pub(crate) content_length: Option<u64>,
    /// Streaming reader over the response body.
    pub(crate) reader: Box<dyn Read>,
}

/// HTTP fetch abstraction so the download logic can be unit-tested with a
/// mock implementation instead of a network connection.
pub(crate) trait Fetcher {
    /// Performs an HTTP GET request.
    ///
    /// # Arguments
    ///
    /// * `url` - Absolute URL to fetch.
    ///
    /// # Returns
    ///
    /// The response status, content length, and body reader. Non-2xx statuses
    /// are returned as data rather than errors; transport failures produce an
    /// `Io` error.
    fn fetch(&self, url: &str) -> LinderaResult<FetchResponse>;
}

/// [`Fetcher`] implementation backed by `ureq`.
struct UreqFetcher {
    /// Configured HTTP agent.
    agent: ureq::Agent,
}

impl UreqFetcher {
    /// Creates a fetcher with the CLI's standard HTTP configuration.
    ///
    /// # Returns
    ///
    /// A fetcher with a 30-second connect timeout, no overall timeout
    /// (archives are up to ~305 MB), and non-2xx statuses passed through as
    /// data (`http_status_as_error(false)`).
    fn new() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(format!("lindera-cli/{}", get_version()))
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();

        Self { agent }
    }
}

impl Fetcher for UreqFetcher {
    /// Performs the GET request with the configured agent, streaming the body
    /// without ureq's default 10 MB read limit.
    ///
    /// # Arguments
    ///
    /// * `url` - Absolute URL to fetch.
    ///
    /// # Returns
    ///
    /// The response status, content length, and streaming body reader.
    fn fetch(&self, url: &str) -> LinderaResult<FetchResponse> {
        let response = self.agent.get(url).call().map_err(io_err)?;
        let status = response.status().as_u16();
        let body = response.into_body();
        let content_length = body.content_length();
        let reader = body.into_with_config().limit(u64::MAX).reader();

        Ok(FetchResponse {
            status,
            content_length,
            reader: Box::new(reader),
        })
    }
}

/// Outcome of a dictionary download request.
#[derive(Debug)]
pub(crate) enum DownloadOutcome {
    /// The dictionary was downloaded and installed at the contained path.
    Installed(PathBuf),
    /// The dictionary was downloaded and replaced a complete one at `path`:
    /// one built by another version of Lindera, or any with `--force`.
    Replaced {
        /// The dictionary directory.
        path: PathBuf,
        /// The version of Lindera that built the replaced dictionary, or
        /// `None` when its `metadata.json` does not say.
        previous_version: Option<String>,
    },
    /// A complete dictionary built by the CLI's version already existed at
    /// the contained path.
    AlreadyPresent(PathBuf),
}

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Download a pre-built dictionary from the Lindera GitHub releases",
    version = get_version(),
)]
/// Arguments for the `download` subcommand.
pub struct DownloadArgs {
    /// Name of the dictionary to download.
    #[clap(
        value_parser = clap::builder::PossibleValuesParser::new(DOWNLOADABLE_DICTIONARIES),
        help = "Dictionary name"
    )]
    name: String,
    /// Replaces an already downloaded dictionary when set.
    #[clap(
        long = "force",
        help = "Re-download and replace an existing dictionary"
    )]
    force: bool,
}

/// Runs the `download` subcommand.
///
/// Downloads the pre-built dictionary archive matching the CLI's own version
/// from the GitHub releases page and installs it in
/// `<LINDERA_DICTIONARIES_DIR>/<name>` (see `dictionaries_dir`). The
/// installed path, the value to pass to `--dict`, is printed to stdout;
/// progress and notices go to stderr.
///
/// # Arguments
///
/// * `args` - Parsed command line arguments.
///
/// # Returns
///
/// `Ok(())` when the dictionary is installed (or already present).
pub fn download(args: DownloadArgs) -> LinderaResult<()> {
    let dictionaries_dir = dictionaries_dir()?;
    let version = get_version();
    let fetcher = UreqFetcher::new();
    let mut progress = progress_renderer(download_url(&args.name, version));

    let outcome = download_dictionary(
        &fetcher,
        &args.name,
        version,
        &dictionaries_dir,
        args.force,
        &mut progress,
    )?;

    if matches!(
        outcome,
        DownloadOutcome::Installed(_) | DownloadOutcome::Replaced { .. }
    ) && io::stderr().is_terminal()
    {
        // Terminate the `\r`-rewritten progress line.
        eprintln!();
    }
    match outcome {
        DownloadOutcome::Installed(path) => {
            eprintln!("Downloaded dictionary '{}'", args.name);
            println!("{}", path.display());
        }
        DownloadOutcome::Replaced {
            path,
            previous_version,
        } => {
            let previous = match previous_version {
                Some(previous) => format!("Lindera {previous}"),
                None => "an unknown version of Lindera".to_string(),
            };
            eprintln!(
                "Downloaded dictionary '{}', replacing the one built by {previous}",
                args.name
            );
            println!("{}", path.display());
        }
        DownloadOutcome::AlreadyPresent(path) => {
            eprintln!(
                "Dictionary '{}' is already downloaded; use --force to re-download it",
                args.name
            );
            println!("{}", path.display());
        }
    }

    Ok(())
}

/// Builds a progress callback that renders download progress on stderr.
///
/// The first invocation prints the URL being downloaded. When stderr is a
/// terminal, subsequent invocations rewrite a single progress line via `\r`
/// (updated on whole-percent changes, or per MiB when the total size is
/// unknown); otherwise no per-chunk output is produced.
///
/// # Arguments
///
/// * `url` - URL shown in the initial progress line.
///
/// # Returns
///
/// A callback taking `(bytes_downloaded, total_bytes)`.
fn progress_renderer(url: String) -> impl FnMut(u64, Option<u64>) {
    /// Converts a byte count to MiB for display.
    fn mib(bytes: u64) -> f64 {
        bytes as f64 / (1024.0 * 1024.0)
    }

    let is_terminal = io::stderr().is_terminal();
    let mut started = false;
    let mut last_rendered: Option<u64> = None;

    move |bytes, total| {
        if !started {
            started = true;
            eprintln!("Downloading {url}");
        }
        if !is_terminal {
            return;
        }
        match total {
            Some(total) if total > 0 => {
                let percent = bytes * 100 / total;
                if last_rendered != Some(percent) {
                    last_rendered = Some(percent);
                    eprint!("\r{percent}% ({:.1}/{:.1} MiB)", mib(bytes), mib(total));
                }
            }
            _ => {
                let whole_mib = bytes / (1024 * 1024);
                if last_rendered != Some(whole_mib) {
                    last_rendered = Some(whole_mib);
                    eprint!("\r{:.1} MiB", mib(bytes));
                }
            }
        }
        let _ = io::stderr().flush();
    }
}

/// Downloads and installs a pre-built dictionary.
///
/// A complete dictionary already in place is kept when its `metadata.json`
/// says that the CLI's version built it, and replaced otherwise, including
/// when it does not say, as for a dictionary built before v7.0.0 (#1303).
/// Streams the release archive to a temporary file, extracts it to a
/// temporary directory, validates the required dictionary files, and
/// atomically renames the result into place. Temporary files are removed on
/// both success and error paths.
///
/// # Arguments
///
/// * `fetcher` - HTTP fetch implementation.
/// * `name` - Downloadable dictionary name.
/// * `version` - Crate version of the running CLI (selects the release tag).
/// * `dictionaries_dir` - The directory that holds the dictionaries.
/// * `force` - Re-download even when a complete dictionary of the CLI's
///   version is present.
/// * `progress` - Callback receiving `(bytes_downloaded, total_bytes)`.
///
/// # Returns
///
/// The installation outcome with the dictionary directory path.
pub(crate) fn download_dictionary(
    fetcher: &dyn Fetcher,
    name: &str,
    version: &str,
    dictionaries_dir: &Path,
    force: bool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> LinderaResult<DownloadOutcome> {
    let target = dictionary_dir(dictionaries_dir, name);
    // The version of a complete dictionary in place, `Some(None)` when its
    // metadata.json does not say.
    let installed = is_complete_dictionary_dir(&target).then(|| installed_lindera_version(&target));
    if !force
        && installed
            .as_ref()
            .is_some_and(|v| v.as_deref() == Some(version))
    {
        return Ok(DownloadOutcome::AlreadyPresent(target));
    }

    fs::create_dir_all(dictionaries_dir).map_err(io_err)?;
    sweep_stale_temps(dictionaries_dir, name);

    let pid = std::process::id();
    let zip_tmp = dictionaries_dir.join(format!(".tmp-lindera-{name}-{pid}.zip"));
    let extract_tmp = dictionaries_dir.join(format!(".tmp-lindera-{name}-{pid}.extract"));

    let result = download_and_install(
        fetcher,
        name,
        version,
        &target,
        &zip_tmp,
        &extract_tmp,
        progress,
    );

    // Clean up temporary files on both success and error paths.
    let _ = fs::remove_file(&zip_tmp);
    let _ = fs::remove_dir_all(&extract_tmp);

    result.map(|path| match installed {
        Some(previous_version) => DownloadOutcome::Replaced {
            path,
            previous_version,
        },
        None => DownloadOutcome::Installed(path),
    })
}

/// Performs the fetch → stream → extract → validate → install sequence.
///
/// # Arguments
///
/// * `fetcher` - HTTP fetch implementation.
/// * `name` - Downloadable dictionary name.
/// * `version` - Crate version of the running CLI.
/// * `target` - Final installation directory.
/// * `zip_tmp` - Temporary file receiving the streamed archive.
/// * `extract_tmp` - Temporary directory receiving the extracted archive.
/// * `progress` - Callback receiving `(bytes_downloaded, total_bytes)`.
///
/// # Returns
///
/// The installed dictionary directory path.
fn download_and_install(
    fetcher: &dyn Fetcher,
    name: &str,
    version: &str,
    target: &Path,
    zip_tmp: &Path,
    extract_tmp: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> LinderaResult<PathBuf> {
    let url = download_url(name, version);
    let response = fetcher.fetch(&url)?;
    match response.status {
        200..=299 => {}
        404 => {
            return Err(LinderaErrorKind::NotFound.with_error(anyhow::anyhow!(
                "dictionary archive not found: {url} (HTTP 404). Version {version} may not have a published release yet; see https://github.com/lindera/lindera/releases"
            )));
        }
        status => {
            return Err(LinderaErrorKind::Content
                .with_error(anyhow::anyhow!("failed to download {url}: HTTP {status}")));
        }
    }

    stream_to_file(response, zip_tmp, progress)?;
    extract_archive(zip_tmp, extract_tmp)?;

    let root = find_dictionary_root(extract_tmp, name)?;
    let missing = missing_dictionary_files(&root);
    if !missing.is_empty() {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "downloaded archive is missing required dictionary files: {}",
            missing.join(", ")
        )));
    }

    install_extracted(&root, target)?;

    Ok(target.to_path_buf())
}

/// Streams a response body to a file in fixed-size chunks.
///
/// # Arguments
///
/// * `response` - Fetch response whose body is streamed.
/// * `dest` - Destination file path (created or truncated).
/// * `progress` - Callback receiving `(bytes_downloaded, total_bytes)`.
///
/// # Returns
///
/// The number of bytes written.
fn stream_to_file(
    response: FetchResponse,
    dest: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> LinderaResult<u64> {
    let FetchResponse {
        content_length,
        mut reader,
        ..
    } = response;

    let file = File::create(dest).map_err(io_err)?;
    let mut writer = BufWriter::new(file);
    let mut buffer = [0u8; CHUNK_SIZE];
    let mut bytes_written: u64 = 0;

    progress(0, content_length);
    loop {
        let read = reader.read(&mut buffer).map_err(io_err)?;
        if read == 0 {
            break;
        }
        writer.write_all(&buffer[..read]).map_err(io_err)?;
        bytes_written += read as u64;
        progress(bytes_written, content_length);
    }
    writer.flush().map_err(io_err)?;

    Ok(bytes_written)
}

/// Extracts a ZIP archive into a directory.
///
/// The zip crate sanitizes entry paths (zip-slip protection) and verifies
/// each entry's CRC32 checksum while reading.
///
/// # Arguments
///
/// * `zip_path` - Path of the ZIP archive.
/// * `extract_dir` - Directory to extract into (created if needed).
fn extract_archive(zip_path: &Path, extract_dir: &Path) -> LinderaResult<()> {
    let file = File::open(zip_path).map_err(io_err)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| {
        LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "failed to read the dictionary archive (the download may be corrupt; try again): {err}"
        ))
    })?;

    archive.extract(extract_dir).map_err(|err| {
        LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "failed to extract the dictionary archive (the download may be corrupt; try again): {err}"
        ))
    })
}

/// Locates the dictionary directory inside an extracted archive.
///
/// Release archives contain a single top-level `lindera-<name>/` directory,
/// but flat archives and archives with a differently named single top-level
/// directory are tolerated for hand-built archives.
///
/// # Arguments
///
/// * `extract_dir` - Directory the archive was extracted into.
/// * `name` - Downloadable dictionary name.
///
/// # Returns
///
/// The directory expected to contain the dictionary files.
fn find_dictionary_root(extract_dir: &Path, name: &str) -> LinderaResult<PathBuf> {
    let expected = extract_dir.join(format!("lindera-{name}"));
    if expected.is_dir() {
        return Ok(expected);
    }

    if is_complete_dictionary_dir(extract_dir) {
        return Ok(extract_dir.to_path_buf());
    }

    let mut dirs = Vec::new();
    for entry in fs::read_dir(extract_dir).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        if entry.file_type().map_err(io_err)?.is_dir() {
            dirs.push(entry.path());
        }
    }
    if let [single] = dirs.as_slice() {
        return Ok(single.clone());
    }

    Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
        "could not locate the dictionary directory inside the downloaded archive"
    )))
}

/// Moves an extracted dictionary directory into its final location.
///
/// # Arguments
///
/// * `root` - Extracted dictionary directory (inside the temporary directory).
/// * `target` - Final installation directory (replaced when present).
fn install_extracted(root: &Path, target: &Path) -> LinderaResult<()> {
    if target.exists() {
        fs::remove_dir_all(target).map_err(io_err)?;
    }

    // `root` (inside a temporary directory there) and `target` are both in
    // the dictionaries directory, so the rename is atomic on the same
    // filesystem.
    match fs::rename(root, target) {
        Ok(()) => Ok(()),
        Err(err) => {
            // A concurrent download may have installed the dictionary between
            // the removal and the rename; accept the winner's result.
            if is_complete_dictionary_dir(target) {
                Ok(())
            } else {
                Err(io_err(err))
            }
        }
    }
}

/// Removes leftover temporary files of interrupted downloads.
///
/// Only entries prefixed with `.tmp-lindera-<name>-` are removed; failures
/// are ignored (best effort). A concurrently running download of the same
/// dictionary may lose its temporary files to this sweep and fail; the
/// winning process still installs a consistent result.
///
/// # Arguments
///
/// * `dictionaries_dir` - The directory that holds the dictionaries.
/// * `name` - Downloadable dictionary name whose temporaries are swept.
fn sweep_stale_temps(dictionaries_dir: &Path, name: &str) {
    let prefix = format!(".tmp-lindera-{name}-");
    let Ok(entries) = fs::read_dir(dictionaries_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if file_name.starts_with(&prefix) {
            let path = entry.path();
            if path.is_dir() {
                let _ = fs::remove_dir_all(&path);
            } else {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::io::Cursor;

    use lindera::dictionary::Metadata;
    use zip::write::SimpleFileOptions;

    use crate::dictionary_registry::REQUIRED_DICTIONARY_FILES;

    use super::*;

    /// Fetcher returning a canned response and counting invocations.
    struct MockFetcher {
        status: u16,
        body: Vec<u8>,
        calls: Cell<usize>,
    }

    impl MockFetcher {
        fn new(status: u16, body: Vec<u8>) -> Self {
            Self {
                status,
                body,
                calls: Cell::new(0),
            }
        }
    }

    impl Fetcher for MockFetcher {
        fn fetch(&self, _url: &str) -> LinderaResult<FetchResponse> {
            self.calls.set(self.calls.get() + 1);
            Ok(FetchResponse {
                status: self.status,
                content_length: Some(self.body.len() as u64),
                reader: Box::new(Cursor::new(self.body.clone())),
            })
        }
    }

    /// Reader failing with an I/O error after yielding some bytes.
    struct FailingReader {
        remaining: usize,
    }

    impl Read for FailingReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::other("simulated mid-stream failure"));
            }
            let n = self.remaining.min(buf.len());
            buf[..n].fill(0);
            self.remaining -= n;
            Ok(n)
        }
    }

    /// Fetcher whose body reader fails mid-stream.
    struct FailingFetcher;

    impl Fetcher for FailingFetcher {
        fn fetch(&self, _url: &str) -> LinderaResult<FetchResponse> {
            Ok(FetchResponse {
                status: 200,
                content_length: Some(1024),
                reader: Box::new(FailingReader { remaining: 512 }),
            })
        }
    }

    /// Builds an in-memory ZIP archive holding the given entries.
    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (path, content) in entries {
            writer.start_file(*path, options).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    /// Builds a valid dictionary ZIP, optionally nested under a directory.
    fn make_dictionary_zip(top_dir: Option<&str>) -> Vec<u8> {
        let entries: Vec<(String, &[u8])> = REQUIRED_DICTIONARY_FILES
            .iter()
            .map(|file| {
                let path = match top_dir {
                    Some(dir) => format!("{dir}/{file}"),
                    None => (*file).to_string(),
                };
                (path, b"test".as_slice())
            })
            .collect();
        let borrowed: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(path, content)| (path.as_str(), *content))
            .collect();
        make_zip(&borrowed)
    }

    /// No-op progress callback for tests.
    fn no_progress() -> impl FnMut(u64, Option<u64>) {
        |_, _| {}
    }

    /// Asserts that no temporary download entries remain in a dictionaries
    /// directory.
    fn assert_no_temps(dictionaries_dir: &Path) {
        if !dictionaries_dir.exists() {
            return;
        }
        for entry in fs::read_dir(dictionaries_dir).unwrap().flatten() {
            let name = entry.file_name();
            assert!(
                !name.to_string_lossy().starts_with(".tmp-lindera-"),
                "leftover temp entry: {name:?}"
            );
        }
    }

    #[test]
    fn happy_path_installs_dictionary() {
        let tmp = tempfile::tempdir().unwrap();
        let fetcher = MockFetcher::new(200, make_dictionary_zip(Some("lindera-ipadic")));
        let outcome = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap();

        let DownloadOutcome::Installed(path) = outcome else {
            panic!("expected Installed outcome");
        };
        assert_eq!(path, tmp.path().join("ipadic"));
        assert!(is_complete_dictionary_dir(&path));
        assert_no_temps(tmp.path());
    }

    /// Writes a complete dictionary into `target`, its `metadata.json` saying
    /// that `version` built it (nothing when `None`), and a marker file that
    /// a download replacing the directory removes.
    fn write_installed(target: &Path, version: Option<&str>) {
        fs::create_dir_all(target).unwrap();
        for file in REQUIRED_DICTIONARY_FILES {
            fs::write(target.join(file), b"existing").unwrap();
        }
        let metadata = Metadata {
            lindera_version: version.map(str::to_string),
            ..Metadata::default()
        };
        fs::write(
            target.join("metadata.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
        fs::write(target.join("marker.txt"), b"marker").unwrap();
    }

    #[test]
    fn already_downloaded_short_circuits_without_fetching() {
        let tmp = tempfile::tempdir().unwrap();
        let target = dictionary_dir(tmp.path(), "ipadic");
        write_installed(&target, Some("9.9.9"));

        let fetcher = MockFetcher::new(200, make_dictionary_zip(Some("lindera-ipadic")));
        let outcome = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap();

        assert!(matches!(outcome, DownloadOutcome::AlreadyPresent(_)));
        assert_eq!(fetcher.calls.get(), 0);
        assert!(target.join("marker.txt").exists());
    }

    #[test]
    fn dictionary_of_another_version_is_downloaded_again() {
        let tmp = tempfile::tempdir().unwrap();
        let target = dictionary_dir(tmp.path(), "ipadic");
        write_installed(&target, Some("9.9.8"));

        let fetcher = MockFetcher::new(200, make_dictionary_zip(Some("lindera-ipadic")));
        let outcome = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap();

        let DownloadOutcome::Replaced {
            path,
            previous_version,
        } = outcome
        else {
            panic!("expected Replaced outcome, got {outcome:?}");
        };
        assert_eq!(path, target);
        assert_eq!(previous_version.as_deref(), Some("9.9.8"));
        assert_eq!(fetcher.calls.get(), 1);
        assert!(!target.join("marker.txt").exists());
    }

    #[test]
    fn dictionary_without_a_version_is_downloaded_again() {
        let tmp = tempfile::tempdir().unwrap();
        let target = dictionary_dir(tmp.path(), "ipadic");
        write_installed(&target, None);

        let fetcher = MockFetcher::new(200, make_dictionary_zip(Some("lindera-ipadic")));
        let outcome = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap();

        assert!(
            matches!(
                outcome,
                DownloadOutcome::Replaced {
                    previous_version: None,
                    ..
                }
            ),
            "{outcome:?}"
        );
        assert_eq!(fetcher.calls.get(), 1);
        assert!(!target.join("marker.txt").exists());
    }

    #[test]
    fn force_replaces_existing_dictionary() {
        let tmp = tempfile::tempdir().unwrap();
        let target = dictionary_dir(tmp.path(), "ipadic");
        write_installed(&target, Some("9.9.9"));

        let fetcher = MockFetcher::new(200, make_dictionary_zip(Some("lindera-ipadic")));
        let outcome = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            true,
            &mut no_progress(),
        )
        .unwrap();

        assert!(
            matches!(
                outcome,
                DownloadOutcome::Replaced {
                    ref previous_version,
                    ..
                } if previous_version.as_deref() == Some("9.9.9")
            ),
            "{outcome:?}"
        );
        assert_eq!(fetcher.calls.get(), 1);
        assert_eq!(fs::read(target.join("metadata.json")).unwrap(), b"test");
        assert!(!target.join("marker.txt").exists());
    }

    #[test]
    fn http_404_reports_missing_release() {
        let tmp = tempfile::tempdir().unwrap();
        let fetcher = MockFetcher::new(404, Vec::new());
        let err = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap_err();

        let message = format!("{err}");
        assert!(message.contains("HTTP 404"), "{message}");
        assert!(
            message.contains("https://github.com/lindera/lindera/releases"),
            "{message}"
        );
    }

    #[test]
    fn other_http_errors_report_status() {
        let tmp = tempfile::tempdir().unwrap();
        let fetcher = MockFetcher::new(500, Vec::new());
        let err = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("HTTP 500"));
    }

    #[test]
    fn mid_stream_failure_leaves_no_remnants() {
        let tmp = tempfile::tempdir().unwrap();
        let err = download_dictionary(
            &FailingFetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap_err();

        assert!(format!("{err}").contains("simulated mid-stream failure"));
        assert!(!dictionary_dir(tmp.path(), "ipadic").exists());
        assert_no_temps(tmp.path());
    }

    #[test]
    fn corrupt_archive_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let fetcher = MockFetcher::new(200, b"this is not a zip archive".to_vec());
        let err = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap_err();

        assert!(format!("{err}").contains("archive"), "{err}");
        assert!(!dictionary_dir(tmp.path(), "ipadic").exists());
        assert_no_temps(tmp.path());
    }

    #[test]
    fn zip_slip_entry_stays_contained() {
        let tmp = tempfile::tempdir().unwrap();
        let mut entries: Vec<(String, &[u8])> = REQUIRED_DICTIONARY_FILES
            .iter()
            .map(|file| (format!("lindera-ipadic/{file}"), b"test".as_slice()))
            .collect();
        entries.push(("../evil.txt".to_string(), b"evil".as_slice()));
        let borrowed: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(path, content)| (path.as_str(), *content))
            .collect();

        let fetcher = MockFetcher::new(200, make_zip(&borrowed));
        let dictionaries_dir = tmp.path().join("dictionaries");
        let result = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            &dictionaries_dir,
            false,
            &mut no_progress(),
        );

        // The zip crate rejects entries escaping the extraction directory, so
        // the download fails; the malicious entry must not land anywhere.
        assert!(result.is_err());
        assert!(!dictionaries_dir.join("evil.txt").exists());
        assert!(!tmp.path().join("evil.txt").exists());
        assert_no_temps(&dictionaries_dir);
    }

    #[test]
    fn flat_archive_is_tolerated() {
        let tmp = tempfile::tempdir().unwrap();
        let fetcher = MockFetcher::new(200, make_dictionary_zip(None));
        let outcome = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap();

        let DownloadOutcome::Installed(path) = outcome else {
            panic!("expected Installed outcome");
        };
        assert!(is_complete_dictionary_dir(&path));
    }

    #[test]
    fn differently_named_single_top_dir_is_tolerated() {
        let tmp = tempfile::tempdir().unwrap();
        let fetcher = MockFetcher::new(200, make_dictionary_zip(Some("custom-name")));
        let outcome = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap();

        let DownloadOutcome::Installed(path) = outcome else {
            panic!("expected Installed outcome");
        };
        assert!(is_complete_dictionary_dir(&path));
        assert_eq!(path, tmp.path().join("ipadic"));
    }

    #[test]
    fn incomplete_archive_lists_missing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let fetcher = MockFetcher::new(
            200,
            make_zip(&[("lindera-ipadic/metadata.json", b"test".as_slice())]),
        );
        let err = download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap_err();

        let message = format!("{err}");
        assert!(
            message.contains("missing required dictionary files"),
            "{message}"
        );
        assert!(message.contains("unk.bin"), "{message}");
    }

    #[test]
    fn progress_receives_content_length() {
        let tmp = tempfile::tempdir().unwrap();
        let body = make_dictionary_zip(Some("lindera-ipadic"));
        let body_len = body.len() as u64;
        let fetcher = MockFetcher::new(200, body);

        let mut final_state = (0u64, None);
        let mut progress = |bytes: u64, total: Option<u64>| {
            final_state = (bytes, total);
        };
        download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut progress,
        )
        .unwrap();

        assert_eq!(final_state, (body_len, Some(body_len)));
    }

    #[test]
    fn stale_temps_are_swept_before_download() {
        let tmp = tempfile::tempdir().unwrap();
        let dictionaries_dir = tmp.path();
        fs::write(
            dictionaries_dir.join(".tmp-lindera-ipadic-12345.zip"),
            b"stale",
        )
        .unwrap();
        fs::create_dir_all(dictionaries_dir.join(".tmp-lindera-ipadic-12345.extract")).unwrap();

        let fetcher = MockFetcher::new(200, make_dictionary_zip(Some("lindera-ipadic")));
        download_dictionary(
            &fetcher,
            "ipadic",
            "9.9.9",
            tmp.path(),
            false,
            &mut no_progress(),
        )
        .unwrap();

        assert_no_temps(dictionaries_dir);
    }
}
