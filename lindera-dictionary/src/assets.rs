use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions, TryLockError, rename};
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use md5::Context;
use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use tar::Archive;
use ureq::Agent;

use crate::LinderaResult;
use crate::builder::DictionaryBuilder;
use crate::dictionary::metadata::{DICTIONARY_FORMAT_VERSION, Metadata};
use crate::error::LinderaErrorKind;

const MAX_ROUND: usize = 3;

pub struct FetchParams {
    /// Dictionary file name
    pub file_name: &'static str,

    /// MeCab directory (archive root directory name)
    pub input_dir: &'static str,

    /// Subdirectory within input_dir to use as the dictionary source.
    /// When `Some("dict-src")`, the dictionary builder reads from `input_dir/dict-src/`
    /// instead of `input_dir/` directly. This is useful when the archive contains
    /// both raw and trained dictionary files in separate directories.
    pub src_subdir: Option<&'static str>,

    /// Lindera directory
    pub output_dir: &'static str,

    /// Dummy lexicon CSV for docs.rs builds.
    ///
    /// Its left and right context ids must be 0: the dummy `matrix.def` is a
    /// 1x1 matrix, and the dictionary build rejects an entry whose ids fall
    /// outside the matrix.
    pub dummy_input: &'static str,

    /// URLs from which to fetch the asset
    pub download_urls: &'static [&'static str],

    /// SHA-256 of the source archive, in hexadecimal digits of either case.
    ///
    /// A downloaded archive, and one already in the build directory, is used
    /// only when it has this digest (#1349). The digest lives in the crate
    /// rather than next to the archive on its host, so that a host or a
    /// mirror that serves another archive cannot also serve a digest for it.
    pub sha256: &'static str,
}

#[cfg(target_os = "windows")]
fn copy_dir_all(src: &Path, dst: &Path) -> LinderaResult<()> {
    if !dst.is_dir() {
        fs::create_dir_all(dst).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to create directory: {dst:?}"))
        })?;
    }

    for entry in fs::read_dir(src).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to read directory: {src:?}"))
    })? {
        let entry = entry.map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to get directory entry in: {src:?}"))
        })?;
        let entry_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &dst_path)?;
        } else {
            fs::copy(&entry_path, &dst_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to copy file: {entry_path:?} to {dst_path:?}"
                    ))
            })?;
        }
    }
    Ok(())
}

/// Removes `dir` with everything in it, if it exists, and creates it empty.
///
/// With [`rename_directory`], this is how a build replaces a dictionary
/// directory, a cache entry included: the new dictionary is written into a
/// temporary directory, the old directory is emptied, and the new one moved
/// in its place. The old files are removed, never written in place, which is
/// what lets a build embed a hard link to a cache entry's file (#1343): the
/// link keeps the old file when the entry is replaced.
///
/// # Arguments
///
/// * `dir` - The directory to empty.
///
/// # Returns
///
/// `Ok(())` once `dir` exists and is empty, or the I/O error.
fn empty_directory(dir: &Path) -> LinderaResult<()> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to remove directory: {dir:?}"))
        })?;
    }

    fs::create_dir_all(dir).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to create directory: {dir:?}"))
    })?;

    Ok(())
}

/// Moves the directory `dir` to `new_dir`, an empty directory or none.
///
/// On Unix it is a `rename`, so the files keep the inodes they were written
/// with in `dir`. On Windows the files are copied into `new_dir` and `dir`
/// is removed, so they are new files there too. Either way, no file already
/// in `new_dir` is written in place (see [`empty_directory`]).
///
/// # Arguments
///
/// * `dir` - The directory to move.
/// * `new_dir` - Where to move it; its parent is created if needed.
///
/// # Returns
///
/// `Ok(())` once the files are in `new_dir`, or the I/O error.
fn rename_directory(dir: &Path, new_dir: &Path) -> LinderaResult<()> {
    // Ensure parent directory of new_dir exists
    if let Some(parent) = new_dir.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to create parent directory: {parent:?}"))
        })?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        rename(dir, new_dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to rename directory: {dir:?} to {new_dir:?}"
                ))
        })?;
    }

    #[cfg(target_os = "windows")]
    {
        copy_dir_all(dir, new_dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!("{err}"))
                .add_context(format!("Failed to copy directory: {dir:?} to {new_dir:?}"))
        })?;

        fs::remove_dir_all(dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to remove source directory after copy: {dir:?}"
                ))
        })?;
    }

    Ok(())
}

/// The `matrix.def` of the dummy dictionary: a 1x1 matrix with its one cell,
/// which covers context id 0, the only id the dummy entries may use (see
/// [`FetchParams::dummy_input`]).
const DUMMY_MATRIX_DEF: &str = "1 1\n0 0 0\n";

/// The `unk.def` of the dummy dictionary: one line for `DEFAULT`, the only
/// category of the dummy `char.def`, since the dictionary builder requires a
/// line for every category, as `mecab-dict-index` does (#1195). Its context
/// id 0 is inside [`DUMMY_MATRIX_DEF`].
const DUMMY_UNK_DEF: &str = "DEFAULT,0,0,0,*\n";

/// Scaffold the dummy dictionary source files used for docs.rs documentation builds.
///
/// The files are created in the directory the dictionary builder actually reads
/// from: `input_dir` itself, or `input_dir/<src_subdir>` when `src_subdir` is
/// `Some`. Directory creation is idempotent so that repeated builds against a
/// reused target directory (as on docs.rs) succeed.
///
/// # Arguments
///
/// * `input_dir` - Root input directory for the dictionary build.
/// * `src_subdir` - Optional subdirectory within `input_dir` that the
///   dictionary builder reads the source files from.
/// * `dummy_input` - Contents of the dummy dictionary CSV file.
///
/// # Returns
///
/// `Ok(())` if all dummy files were created, or a `LinderaError` on I/O failure.
fn create_dummy_dictionary_source(
    input_dir: &Path,
    src_subdir: Option<&str>,
    dummy_input: &str,
) -> LinderaResult<()> {
    // The dictionary builder reads from `input_dir/<src_subdir>` when set, so
    // the dummy files must be scaffolded there as well.
    let dummy_src_dir = match src_subdir {
        Some(subdir) => input_dir.join(subdir),
        None => input_dir.to_path_buf(),
    };

    // `create_dir_all` keeps this idempotent: docs.rs reuses the target
    // directory, so the directory may already exist from a previous run.
    fs::create_dir_all(&dummy_src_dir).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to create dummy input directory: {dummy_src_dir:?}"
            ))
    })?;

    // Create dummy char.def
    let mut dummy_char_def = File::create(dummy_src_dir.join("char.def")).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to create dummy char.def: {:?}",
                dummy_src_dir.join("char.def")
            ))
    })?;
    dummy_char_def
        .write_all(b"DEFAULT 0 1 0\n")
        .map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to write to dummy char.def")
        })?;

    // Create dummy CSV file
    let mut dummy_dict_csv = File::create(dummy_src_dir.join("dummy_dict.csv")).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to create dummy CSV file: {:?}",
                dummy_src_dir.join("dummy_dict.csv")
            ))
    })?;
    dummy_dict_csv
        .write_all(dummy_input.as_bytes())
        .map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to write to dummy CSV file")
        })?;

    // Create dummy unk.def
    let mut dummy_unk_def = File::create(dummy_src_dir.join("unk.def")).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to create dummy unk.def: {:?}",
                dummy_src_dir.join("unk.def")
            ))
    })?;
    dummy_unk_def
        .write_all(DUMMY_UNK_DEF.as_bytes())
        .map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to write to dummy unk.def")
        })?;

    // Create dummy matrix.def
    let mut dummy_matrix_def = File::create(dummy_src_dir.join("matrix.def")).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to create dummy matrix.def: {:?}",
                dummy_src_dir.join("matrix.def")
            ))
    })?;
    dummy_matrix_def
        .write_all(DUMMY_MATRIX_DEF.as_bytes())
        .map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to write to dummy matrix.def")
        })?;

    Ok(())
}

/// Dictionary archives run to hundreds of megabytes, so the 10 MB default body
/// limit does not apply here. The digest check below is what guards against a
/// truncated or substituted download.
const BODY_LIMIT: u64 = u64::MAX;

/// Checks a digest against the SHA-256 that a dictionary crate pins, for its
/// source archive ([`FetchParams::sha256`]) or its published dictionary
/// ([`PrebuiltDictionary::sha256`]).
///
/// # Arguments
///
/// * `expected` - The pinned SHA-256, in hexadecimal digits of either case.
/// * `actual` - The SHA-256 of the file, from [`sha256_hex`] or
///   [`file_sha256`].
///
/// # Returns
///
/// `Ok(())` when the digests match, ignoring the case of the hexadecimal
/// digits, or a message that names both digests.
fn check_sha256(expected: &str, actual: &str) -> Result<(), String> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "SHA-256 mismatch: expected {expected}, got {actual}"
        ))
    }
}

/// Computes the SHA-256 of `content`.
///
/// # Arguments
///
/// * `content` - The bytes to hash.
///
/// # Returns
///
/// The digest as lowercase hexadecimal digits.
fn sha256_hex(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    hex_digits(&Sha256::digest(content))
}

/// Writes `bytes` as lowercase hexadecimal digits, two per byte.
///
/// # Arguments
///
/// * `bytes` - The bytes of a digest.
///
/// # Returns
///
/// The digits.
fn hex_digits(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Downloads a file from one of `download_urls` and checks its SHA-256.
///
/// Each round tries every URL once, in a random order, and a response that
/// fails (a request error, a non-2xx status, a body that cannot be read or a
/// digest that does not match) moves on to the next URL.
///
/// # Arguments
///
/// * `agent` - The HTTP agent, with `http_status_as_error` off.
/// * `download_urls` - The URLs to try.
/// * `max_rounds` - How many times to go through the URLs.
/// * `expected_sha256` - The SHA-256 the file must have ([`check_sha256`]).
///
/// # Returns
///
/// The content of the first response with the expected digest, or an error
/// that gives the reason the last attempt failed.
fn download_with_retry(
    agent: &Agent,
    download_urls: Vec<&str>,
    max_rounds: usize,
    expected_sha256: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if download_urls.is_empty() {
        return Err("No download URLs provided".into());
    }

    // The reason the latest attempt failed, for the final error.
    let mut last_failure = String::new();
    for round in 0..max_rounds {
        let mut urls = download_urls.clone();

        let mut rng = SmallRng::seed_from_u64(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
        );
        urls.shuffle(&mut rng);

        debug!(
            "Round {}/{}: Trying {} URLs",
            round + 1,
            max_rounds,
            urls.len()
        );

        for url in urls {
            debug!("Attempting to download from {url}");
            match agent.get(url).call() {
                Ok(resp) if resp.status().is_success() => {
                    debug!("HTTP download successful from {url}");

                    match resp
                        .into_body()
                        .with_config()
                        .limit(BODY_LIMIT)
                        .read_to_vec()
                    {
                        Ok(content) => match check_sha256(expected_sha256, &sha256_hex(&content)) {
                            Ok(()) => {
                                debug!("Digest check passed from {url}");
                                return Ok(content);
                            }
                            Err(mismatch) => {
                                warn!("{mismatch} from {url}");
                                last_failure = format!("{mismatch} from {url}");
                                // continue to next url
                            }
                        },
                        Err(e) => {
                            warn!("Failed to download content from {url}: {e}");
                            last_failure = format!("failed to read the response from {url}: {e}");
                            // continue to next url
                        }
                    }
                }
                Ok(resp) => {
                    warn!("HTTP download failed from {}: HTTP {}", url, resp.status());
                    last_failure = format!("HTTP {} from {url}", resp.status());
                    // continue to next url
                }
                Err(e) => {
                    warn!("Request error from {url}: {e}");
                    last_failure = format!("request to {url} failed: {e}");
                    // continue to next url
                }
            }
        }

        // No pause after the last round, which only reports the failure.
        if round + 1 < max_rounds {
            sleep(Duration::from_secs(1));
        }
    }

    error!("All {max_rounds} attempts failed");
    Err(format!(
        "Failed to download a valid file from all sources in {max_rounds} rounds; \
         the last attempt: {last_failure}"
    )
    .into())
}

/// Environment variable that designates the dictionary build cache directory.
const CACHE_DIR_ENV: &str = "LINDERA_BUILD_DICTIONARY_CACHE_DIR";

/// Environment variable that docs.rs sets, under which the build scripts
/// generate dummy dictionaries instead of downloading the real ones.
const DOCS_RS_ENV: &str = "DOCS_RS";

/// Whether this is a documentation build, which uses dummy dictionaries.
///
/// # Returns
///
/// `true` when [`DOCS_RS_ENV`] is set.
fn is_docs_rs_build() -> bool {
    std::env::var(DOCS_RS_ENV).is_ok()
}

/// Reads the dictionary build cache directory from the environment.
///
/// # Returns
///
/// The configured cache directory, or `None` when [`CACHE_DIR_ENV`] is not set
/// or when this is a documentation build; see [`cache_dir_unless_docs_rs`].
fn dictionary_cache_dir_from_env() -> Option<OsString> {
    cache_dir_unless_docs_rs(std::env::var_os(CACHE_DIR_ENV), is_docs_rs_build())
}

/// The dictionary build cache directory to use, given the environment.
///
/// A documentation build generates dummy dictionaries, which must stay out of
/// the cache: a dummy built there would carry the same record of its inputs
/// and the same `metadata.json` as a real build, so [`cache_decision`] would
/// accept it, and a later build without `DOCS_RS` would embed the dummy
/// (#1157). A documentation
/// build therefore ignores the cache, as if the variable were unset, which is
/// how docs.rs runs it: the dummies go into `OUT_DIR`, and a crate whose embed
/// feature is off builds nothing.
///
/// # Arguments
///
/// * `cache_dir` - The value of [`CACHE_DIR_ENV`], if set.
/// * `docs_rs` - Whether this is a documentation build ([`DOCS_RS_ENV`] is
///   set).
///
/// # Returns
///
/// `cache_dir`, or `None` when `docs_rs` is `true`.
fn cache_dir_unless_docs_rs(cache_dir: Option<OsString>, docs_rs: bool) -> Option<OsString> {
    cache_dir.filter(|_| !docs_rs)
}

/// Environment variable naming a context-ID frequency histogram that a
/// dictionary crate's build reads in place of the bundled
/// [`CONTEXT_ID_FREQ_FILE`], when `connection_id_mapping` is enabled.
///
/// Only the build scripts read it ([`fetch_for`]): [`DictionaryBuilder`]
/// reads no environment variable, so `lindera build` takes the histogram from
/// its `--context-id-freq` option alone (#1308).
const CONTEXT_ID_FREQ_FILE_ENV: &str = "LINDERA_BUILD_CONTEXT_ID_FREQ_FILE";

/// The former name of [`CONTEXT_ID_FREQ_FILE_ENV`], read with a deprecation
/// warning when that is not set, until v8.0.0 (#1308).
const DEPRECATED_CTX_FREQ_FILE_ENV: &str = "LINDERA_CTX_FREQ_FILE";

/// The warning for a histogram named by [`DEPRECATED_CTX_FREQ_FILE_ENV`]
/// alone. Cargo shows it after `warning: <crate>@<version>:`, so it does not
/// start with `Warning:` as the runtime warnings of the other renames do.
const CTX_FREQ_FILE_DEPRECATED_WARNING: &str = "LINDERA_CTX_FREQ_FILE is deprecated and will be removed in v8.0.0; set LINDERA_BUILD_CONTEXT_ID_FREQ_FILE instead";

/// The warning for [`DEPRECATED_CTX_FREQ_FILE_ENV`] set next to
/// [`CONTEXT_ID_FREQ_FILE_ENV`].
const CTX_FREQ_FILE_IGNORED_WARNING: &str = "LINDERA_CTX_FREQ_FILE is ignored because LINDERA_BUILD_CONTEXT_ID_FREQ_FILE is set; LINDERA_CTX_FREQ_FILE is deprecated and will be removed in v8.0.0";

/// The histogram a dictionary crate's build reads in place of the bundled
/// one, and the warning it prints about [`DEPRECATED_CTX_FREQ_FILE_ENV`].
#[derive(Debug, PartialEq, Eq)]
struct ContextIdFreqFileChoice {
    /// The histogram to read, or `None` to keep the bundled one, if any.
    path: Option<PathBuf>,
    /// The warning to print, if the deprecated variable is set.
    warning: Option<&'static str>,
}

/// Chooses the histogram that replaces the bundled one from the values of
/// [`CONTEXT_ID_FREQ_FILE_ENV`] and [`DEPRECATED_CTX_FREQ_FILE_ENV`]. The
/// caller reads the environment, so that every combination can be tested
/// without changing it.
///
/// # Arguments
///
/// * `context_id_freq_file` - The value of
///   `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE`, if set.
/// * `ctx_freq_file` - The value of `LINDERA_CTX_FREQ_FILE`, if set.
///
/// # Returns
///
/// `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` when it is set, with a warning that
/// `LINDERA_CTX_FREQ_FILE` is ignored if that is set too; otherwise
/// `LINDERA_CTX_FREQ_FILE`, with a deprecation warning; and no file when
/// neither is set.
fn choose_context_id_freq_file(
    context_id_freq_file: Option<OsString>,
    ctx_freq_file: Option<OsString>,
) -> ContextIdFreqFileChoice {
    match (context_id_freq_file, ctx_freq_file) {
        (Some(file), old) => ContextIdFreqFileChoice {
            path: Some(PathBuf::from(file)),
            warning: old.map(|_| CTX_FREQ_FILE_IGNORED_WARNING),
        },
        (None, Some(old)) => ContextIdFreqFileChoice {
            path: Some(PathBuf::from(old)),
            warning: Some(CTX_FREQ_FILE_DEPRECATED_WARNING),
        },
        (None, None) => ContextIdFreqFileChoice {
            path: None,
            warning: None,
        },
    }
}

/// Attaches the histogram chosen from the environment to `builder`, in place
/// of the bundled one.
///
/// # Arguments
///
/// * `builder` - The dictionary builder, with the bundled histogram if the
///   crate ships one.
/// * `path` - The histogram chosen by [`choose_context_id_freq_file`], if any.
///
/// # Returns
///
/// `builder` reading `path` when it is given, and `builder` unchanged
/// otherwise.
fn with_context_id_freq_override(
    builder: DictionaryBuilder,
    path: Option<PathBuf>,
) -> DictionaryBuilder {
    match path {
        Some(path) => builder.with_context_id_freq(path),
        None => builder,
    }
}

/// Name of the file, in a dictionary crate's root next to `metadata.json`, that
/// pins the crate's published dictionary ([`PrebuiltDictionary`], #1336).
const PREBUILT_FILE: &str = "prebuilt.json";

/// Environment variable that makes a dictionary crate build its dictionary
/// from source even when it pins a published one (#1336).
const FROM_SOURCE_ENV: &str = "LINDERA_BUILD_DICTIONARY_FROM_SOURCE";

/// The files of a built dictionary that `embedded_dictionary!` includes.
const DICTIONARY_FILES: [&str; 9] = [
    "char_def.bin",
    "matrix.mtx",
    "dict.trie",
    "dict.valsidx",
    "dict.vals",
    "dict.wordsidx",
    "dict.words",
    "unk.bin",
    "metadata.json",
];

/// A dictionary crate's published dictionary, as its [`PREBUILT_FILE`] pins
/// it.
///
/// The official dictionaries are built once per change of their inputs and
/// published as GitHub releases, one per artifact; an `embed-*` build
/// downloads the pinned archive and checks its SHA-256 instead of building the
/// dictionary from source ([`choose_dictionary_source`]).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrebuiltDictionary {
    /// Name of the artifact: the dictionary, the version of its source, the
    /// dictionary format version and a revision, such as
    /// `ipadic-2.7.0-20250920-fmt3-r1`. The archive holds one directory of
    /// that name.
    key: String,
    /// The dictionary format version the artifact was built in.
    format_version: u32,
    /// The URL the archive's path is relative to; kept apart from `path` so
    /// that a mirror can replace it (#584).
    base_url: String,
    /// The path of the archive under `base_url`.
    path: String,
    /// The SHA-256 of the archive, or `None` until it is published; without
    /// it, the dictionary is built from source.
    sha256: Option<String>,
}

impl PrebuiltDictionary {
    /// The URL of the archive.
    ///
    /// # Returns
    ///
    /// `base_url` and `path` joined by a single `/`.
    fn url(&self) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            self.path.trim_start_matches('/')
        )
    }
}

/// Reads a dictionary crate's [`PREBUILT_FILE`].
///
/// # Arguments
///
/// * `path` - The file to read.
///
/// # Returns
///
/// The pin, `None` when the file does not exist, or an error when it cannot
/// be read or parsed.
fn read_prebuilt(path: &Path) -> LinderaResult<Option<PrebuiltDictionary>> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to read {path:?}")));
        }
    };
    serde_json::from_str(&data).map(Some).map_err(|err| {
        LinderaErrorKind::Deserialize
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to parse {path:?}"))
    })
}

/// Reads [`FROM_SOURCE_ENV`].
///
/// # Arguments
///
/// * `value` - The value of the variable, if set.
///
/// # Returns
///
/// Whether the build from source is forced: `true` for `1` and `true` (in any
/// case), `false` when the variable is unset, empty, `0` or `false`. Any
/// other value is taken as `true`, with a warning to print.
fn from_source_requested(value: Option<OsString>) -> (bool, Option<String>) {
    let Some(value) = value else {
        return (false, None);
    };
    let text = value.to_string_lossy();
    match text.trim().to_ascii_lowercase().as_str() {
        "1" | "true" => (true, None),
        "" | "0" | "false" => (false, None),
        _ => (
            true,
            Some(format!(
                "{FROM_SOURCE_ENV}={text} is taken as 1; set it to 1 or true to build the \
                 dictionary from source, or to 0 or false (or unset it) not to"
            )),
        ),
    }
}

/// How a build gets the dictionary it embeds.
#[derive(Clone, Debug, PartialEq, Eq)]
enum DictionarySource {
    /// A dummy dictionary from [`FetchParams::dummy_input`], for docs.rs.
    Dummy,
    /// The published dictionary, downloaded from `url` and checked against
    /// `sha256`.
    Published {
        /// The artifact's name ([`PrebuiltDictionary::key`]).
        key: String,
        /// Where to download it.
        url: String,
        /// The SHA-256 it must have.
        sha256: String,
    },
    /// Built from the source archive. `reason` says why the published
    /// dictionary is not used, when the crate pins one; the build prints it
    /// as a `cargo:warning`.
    Source {
        /// Why the published dictionary is not used, if one is pinned.
        reason: Option<String>,
    },
}

/// Decides how a build gets its dictionary (#1336).
///
/// A build uses the published dictionary when the crate pins one with its
/// SHA-256 and nothing asks for another: a docs.rs build uses a dummy, as it
/// has no network; a build that forces the build from source
/// ([`FROM_SOURCE_ENV`]), one that ranks context IDs with another histogram
/// ([`CONTEXT_ID_FREQ_FILE_ENV`]), whose output differs from the published
/// dictionary, and one whose format version differs from the pinned one, as
/// on `main` between a format change and the publication of new dictionaries,
/// build from source.
///
/// # Arguments
///
/// * `docs_rs` - Whether this is a docs.rs build ([`DOCS_RS_ENV`]).
/// * `from_source` - Whether [`FROM_SOURCE_ENV`] asks for the build from
///   source.
/// * `context_id_freq_override` - Whether a histogram other than the bundled
///   one is set.
/// * `prebuilt` - The crate's pin, if it has a [`PREBUILT_FILE`].
/// * `name` - The dictionary's name, for the reason.
///
/// # Returns
///
/// The way to get the dictionary.
fn choose_dictionary_source(
    docs_rs: bool,
    from_source: bool,
    context_id_freq_override: bool,
    prebuilt: Option<&PrebuiltDictionary>,
    name: &str,
) -> DictionarySource {
    if docs_rs {
        return DictionarySource::Dummy;
    }
    let Some((prebuilt, sha256)) =
        prebuilt.and_then(|prebuilt| prebuilt.sha256.as_ref().map(|sha256| (prebuilt, sha256)))
    else {
        return DictionarySource::Source { reason: None };
    };
    let reason = if from_source {
        Some(format!(
            "building the {name} dictionary from source, because {FROM_SOURCE_ENV} is set"
        ))
    } else if context_id_freq_override {
        Some(format!(
            "building the {name} dictionary from source, because {CONTEXT_ID_FREQ_FILE_ENV} \
             names another context-ID frequency file, with which the dictionary differs from \
             the published one"
        ))
    } else if prebuilt.format_version != DICTIONARY_FORMAT_VERSION {
        Some(format!(
            "building the {name} dictionary from source: the published dictionary {} has \
             format version {}, and this build reads format version {DICTIONARY_FORMAT_VERSION}",
            prebuilt.key, prebuilt.format_version
        ))
    } else {
        None
    };
    match reason {
        Some(reason) => DictionarySource::Source {
            reason: Some(reason),
        },
        None => DictionarySource::Published {
            key: prebuilt.key.clone(),
            url: prebuilt.url(),
            sha256: sha256.clone(),
        },
    }
}

/// Name of the file, inside a cached dictionary's output directory, that
/// records the inputs the dictionary was built from ([`BuildInputs`]).
///
/// It lives inside the output directory rather than next to it, so that any
/// code that rebuilds the dictionary -- including releases of this crate from
/// before the file existed, which empty the directory first -- also removes
/// the record, and a record never describes output it did not come with.
const BUILD_INPUTS_FILE: &str = "build_inputs.json";

/// Digest of this crate's source, computed by `build.rs` with the `build_rs`
/// feature; `None` when it could not read the source.
const BUILDER_SOURCE_DIGEST: Option<&str> = option_env!("LINDERA_DICTIONARY_SOURCE_DIGEST");

/// Prefix of the lines a build script prints about the dictionary cache.
///
/// They are plain stdout lines, not `cargo:` directives: cargo keeps them in
/// the build script's `output` file and shows them with `cargo build -vv`.
const CACHE_REPORT_PREFIX: &str = "lindera dictionary cache:";

/// The inputs that decide the content of a built dictionary, apart from the
/// crate's `metadata.json` and the dictionary format version, which the cache
/// checks separately ([`check_cached_metadata`]).
///
/// A cached dictionary is reused only when the record written next to it
/// equals the inputs of the current build. Versions of the builder's
/// dependencies are not recorded: a dependency upgrade that changes the
/// output must bump [`DICTIONARY_FORMAT_VERSION`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildInputs {
    /// Version of the `lindera-dictionary` crate that built the dictionary.
    builder_version: String,
    /// Digest of that crate's source ([`BUILDER_SOURCE_DIGEST`]).
    builder_digest: String,
    /// File name of the source archive ([`FetchParams::file_name`]).
    archive_file_name: String,
    /// SHA-256 of the source archive ([`FetchParams::sha256`]), in lowercase.
    /// It replaced the archive's MD5 (#1349): a record that still has
    /// `archive_md5` does not parse, so its entry is rebuilt.
    archive_sha256: String,
    /// Directory the archive extracts to ([`FetchParams::input_dir`]).
    input_dir: String,
    /// Sub-directory of the extracted archive the build reads
    /// ([`FetchParams::src_subdir`]).
    src_subdir: Option<String>,
    /// MD5 of the context-ID frequency histogram the build ranks context IDs
    /// with, or `None` when it reads none (`connection_id_mapping` off, or no
    /// histogram found).
    context_id_freq_md5: Option<String>,
    /// The published dictionary the entry was downloaded as, or `None` when
    /// it was built from source (#1336). Always written, also as `null`, so
    /// that a build from source never takes an entry that was downloaded,
    /// and the other way round. A record from before the field reads as
    /// `None`, since serde reads a missing `Option` field as `None`.
    published: Option<PublishedRecord>,
}

/// The published dictionary that a cache entry was downloaded as.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishedRecord {
    /// The artifact's name ([`PrebuiltDictionary::key`]).
    key: String,
    /// The SHA-256 of its archive.
    sha256: String,
}

/// What a build does with the dictionary it builds into the cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CachePurpose {
    /// The dictionary is embedded, so only a current cache entry may be used.
    Embed,
    /// The crate's embed feature is off and the build only fills the cache,
    /// so an existing entry is kept as it is. A build with the embed feature
    /// on checks it before using it.
    Prefill,
}

/// Why a cached dictionary is used.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CacheHit {
    /// Its recorded inputs and its `metadata.json` match the current build.
    Current,
    /// The build only fills the cache ([`CachePurpose::Prefill`]), and the
    /// entry exists.
    Kept,
}

/// Why a cached dictionary is not used.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CacheMiss {
    /// There is no cached dictionary yet.
    Empty,
    /// This crate's source digest is unavailable, so no entry can be shown to
    /// come from the current builder.
    DigestUnavailable,
    /// The entry has no record of its inputs: it was built by a release from
    /// before the record existed, or its build was interrupted.
    NoRecord,
    /// The record cannot be read or parsed.
    UnreadableRecord,
    /// The named recorded inputs differ from the current build's.
    InputsChanged(Vec<String>),
    /// The cached `metadata.json` cannot be read or parsed.
    UnreadableMetadata,
    /// The cached dictionary declares another dictionary format version.
    FormatVersion(u32),
    /// The cached `metadata.json` differs from the crate's.
    MetadataChanged,
}

impl std::fmt::Display for CacheHit {
    /// Describes the hit for [`CACHE_REPORT_PREFIX`] lines.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    ///
    /// # Returns
    ///
    /// The result of writing to `f`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheHit::Current => write!(f, "the inputs and metadata.json are unchanged"),
            CacheHit::Kept => write!(
                f,
                "the embed feature is off, so the entry is kept without a check"
            ),
        }
    }
}

impl std::fmt::Display for CacheMiss {
    /// Describes the miss for [`CACHE_REPORT_PREFIX`] lines.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    ///
    /// # Returns
    ///
    /// The result of writing to `f`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheMiss::Empty => write!(f, "nothing is cached yet"),
            CacheMiss::DigestUnavailable => {
                write!(f, "the lindera-dictionary source digest is unavailable")
            }
            CacheMiss::NoRecord => write!(f, "{BUILD_INPUTS_FILE} is missing"),
            CacheMiss::UnreadableRecord => write!(f, "{BUILD_INPUTS_FILE} cannot be read"),
            CacheMiss::InputsChanged(fields) => write!(f, "changed: {}", fields.join(", ")),
            CacheMiss::UnreadableMetadata => write!(f, "metadata.json cannot be read"),
            CacheMiss::FormatVersion(found) => write!(
                f,
                "dictionary format version {found}, expected {DICTIONARY_FORMAT_VERSION}"
            ),
            CacheMiss::MetadataChanged => {
                write!(f, "metadata.json differs from the crate's")
            }
        }
    }
}

/// Computes the MD5 of a file, reading it in chunks.
///
/// # Arguments
///
/// * `path` - The file to hash.
///
/// # Returns
///
/// The digest as lowercase hexadecimal digits, or the I/O error.
fn file_md5(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut context = Context::new();
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.consume(&buffer[..count]);
    }
    Ok(format!("{:x}", context.finalize()))
}

/// Computes the SHA-256 of a file, reading it in chunks.
///
/// # Arguments
///
/// * `path` - The file to hash.
///
/// # Returns
///
/// The digest as lowercase hexadecimal digits, or the I/O error.
fn file_sha256(path: &Path) -> io::Result<String> {
    use sha2::{Digest, Sha256};

    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex_digits(&hasher.finalize()))
}

/// The inputs of the build `fetch` is about to run, to compare with a cached
/// dictionary's record.
///
/// # Arguments
///
/// * `params` - The source archive and directory layout of the build.
/// * `builder` - The builder, for its metadata and the histogram it reads:
///   the bundled one, or the one [`fetch_for`] attached in its place.
/// * `builder_digest` - This crate's source digest ([`BUILDER_SOURCE_DIGEST`]).
/// * `source` - How the build gets the dictionary; a published dictionary is
///   recorded by its key and SHA-256.
///
/// # Returns
///
/// The inputs, `None` when `builder_digest` is `None`, or an error when the
/// histogram the build would read cannot be read.
fn expected_build_inputs(
    params: &FetchParams,
    builder: &DictionaryBuilder,
    builder_digest: Option<&str>,
    source: &DictionarySource,
) -> LinderaResult<Option<BuildInputs>> {
    let Some(builder_digest) = builder_digest else {
        return Ok(None);
    };
    let context_id_freq_md5 = if builder.metadata().connection_id_mapping {
        builder
            .context_id_freq()
            .map(|path| {
                file_md5(path).map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context(format!(
                            "Failed to read the context-id frequency file: {path:?}"
                        ))
                })
            })
            .transpose()?
    } else {
        None
    };
    Ok(Some(BuildInputs {
        builder_version: env!("CARGO_PKG_VERSION").to_string(),
        builder_digest: builder_digest.to_string(),
        archive_file_name: params.file_name.to_string(),
        archive_sha256: params.sha256.to_ascii_lowercase(),
        input_dir: params.input_dir.to_string(),
        src_subdir: params.src_subdir.map(str::to_string),
        context_id_freq_md5,
        published: match source {
            DictionarySource::Published { key, sha256, .. } => Some(PublishedRecord {
                key: key.clone(),
                sha256: sha256.to_ascii_lowercase(),
            }),
            DictionarySource::Dummy | DictionarySource::Source { .. } => None,
        },
    }))
}

/// Names the inputs on which two records differ.
///
/// The comparison goes through the JSON representation, so a field added to
/// [`BuildInputs`] is compared without changes here.
///
/// # Arguments
///
/// * `recorded` - The record of a cached dictionary.
/// * `expected` - The inputs of the current build.
///
/// # Returns
///
/// The names of the differing fields; empty when the records are equal.
fn changed_inputs(recorded: &BuildInputs, expected: &BuildInputs) -> Vec<String> {
    let (Ok(serde_json::Value::Object(recorded)), Ok(serde_json::Value::Object(expected))) = (
        serde_json::to_value(recorded),
        serde_json::to_value(expected),
    ) else {
        return vec!["the whole record".to_string()];
    };
    expected
        .iter()
        .filter(|(field, value)| recorded.get(field.as_str()) != Some(value))
        .map(|(field, _)| field.clone())
        .collect()
}

/// The path of the record of a cached dictionary's inputs.
///
/// # Arguments
///
/// * `output_dir` - The cached dictionary's output directory.
///
/// # Returns
///
/// The path of [`BUILD_INPUTS_FILE`] in `output_dir`.
fn build_inputs_path(output_dir: &Path) -> PathBuf {
    output_dir.join(BUILD_INPUTS_FILE)
}

/// Suffix of the lock file a build holds while it writes a dictionary's cache
/// entry, named after the entry's directory ([`lock_cache_entry`]).
const CACHE_LOCK_SUFFIX: &str = ".lock";

/// Takes the lock on a dictionary's entry in the cache, waiting while another
/// build holds it.
///
/// A build that writes the entry uses fixed paths next to it -- the download,
/// the extracted archive, the input and output directories -- so two builds of
/// the same dictionary that share the cache remove each other's files (#1206).
/// Holding this lock from the cache check to the record lets one of them
/// write at a time. It is an advisory lock ([`File::lock`]: `flock` on Unix,
/// `LockFileEx` on Windows) on `<build_dir>/<output_dir>.lock`, released when
/// the returned file is dropped or the process exits, so a build that is
/// killed leaves no stale lock. The file stays empty and is never removed.
///
/// When the lock file cannot be opened, or the platform or file system cannot
/// lock it, the build goes on without the lock, as builds did before it
/// existed, after a `cargo:warning`. Once the lock is found taken, though,
/// another build is known to be writing, so an error while waiting for it is
/// returned instead.
///
/// # Arguments
///
/// * `build_dir` - The cache directory holding the entry.
/// * `output_dir` - The entry's directory name ([`FetchParams::output_dir`]).
///
/// # Returns
///
/// The locked file, which holds the lock until it is dropped, or `None` when
/// the build goes on without the lock; or the error that ended the wait.
fn lock_cache_entry(build_dir: &Path, output_dir: &str) -> LinderaResult<Option<File>> {
    let path = build_dir.join(format!("{output_dir}{CACHE_LOCK_SUFFIX}"));
    let without_lock = |err: &dyn std::fmt::Display| {
        println!(
            "cargo:warning=cannot lock {}: {err}; building without the lock, so a concurrent \
             build of the same dictionary sharing the cache can collide with this one",
            path.display()
        );
    };
    // Open for writing as well: whether a file open only for reading can be
    // locked is unspecified, and some platforms refuse it.
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
    {
        Ok(file) => file,
        Err(err) => {
            without_lock(&err);
            return Ok(None);
        }
    };
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => {
            println!(
                "{CACHE_REPORT_PREFIX} waiting for another build ({})",
                path.display()
            );
            loop {
                match file.lock() {
                    Ok(()) => return Ok(Some(file)),
                    // A signal that arrives while `lock` waits ends the wait
                    // with `Interrupted` instead of resuming it.
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) => {
                        return Err(LinderaErrorKind::Io
                            .with_error(anyhow::anyhow!(err))
                            .add_context(format!("Failed to wait for the lock {path:?}")));
                    }
                }
            }
        }
        Err(TryLockError::Error(err)) => {
            without_lock(&err);
            Ok(None)
        }
    }
}

/// Records the inputs of a dictionary just built into the cache.
///
/// `fetch` calls this after moving the dictionary into place, so an
/// interrupted build leaves no record, and the next build rebuilds.
///
/// # Arguments
///
/// * `output_dir` - The output directory of the dictionary.
/// * `inputs` - The inputs it was built from.
///
/// # Returns
///
/// `Ok(())`, or an error when the record cannot be serialized or written.
fn write_build_inputs(output_dir: &Path, inputs: &BuildInputs) -> LinderaResult<()> {
    let path = build_inputs_path(output_dir);
    let data = serde_json::to_vec_pretty(inputs).map_err(|err| {
        LinderaErrorKind::Serialize
            .with_error(anyhow::anyhow!(err))
            .add_context("Failed to serialize the dictionary build inputs")
    })?;
    fs::write(&path, data).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to write {path:?}"))
    })
}

/// Whether a cached dictionary directory was built in the format this crate
/// reads, from the metadata this crate ships.
///
/// The cache directory is keyed on the crate version and the dictionary
/// format version only, so a change to the crate's `metadata.json` that bumps
/// neither (a new schema flag, newly shipped rules) would otherwise be served
/// from a warm cache: `cargo:rerun-if-changed=metadata.json` reruns the build
/// script, but the cached artifacts, `metadata.json` included, would be
/// embedded verbatim. Comparing against `source` closes that gap.
///
/// # Arguments
///
/// * `output_dir` - The cached dictionary directory to inspect.
/// * `source` - The metadata the crate ships (`metadata.json` in the crate
///   root), which the cached copy must match apart from the fields the build
///   derives; see [`metadata_matches_source`].
///
/// # Returns
///
/// `Ok(())` only when `metadata.json` is present, parses, declares
/// [`DICTIONARY_FORMAT_VERSION`], and matches `source`; otherwise the reason.
fn check_cached_metadata(output_dir: &Path, source: &Metadata) -> Result<(), CacheMiss> {
    let data =
        fs::read(output_dir.join("metadata.json")).map_err(|_| CacheMiss::UnreadableMetadata)?;
    let metadata =
        serde_json::from_slice::<Metadata>(&data).map_err(|_| CacheMiss::UnreadableMetadata)?;
    if metadata.format_version != DICTIONARY_FORMAT_VERSION {
        return Err(CacheMiss::FormatVersion(metadata.format_version));
    }
    if !metadata_matches_source(&metadata, source) {
        return Err(CacheMiss::MetadataChanged);
    }
    Ok(())
}

/// Decides whether the dictionary cached in `output_dir` may be used.
///
/// Anything unexpected -- a missing or unreadable record or `metadata.json`,
/// an input or a version that does not match -- is a miss, so the caller
/// rebuilds. A cache is an optimization; refusing to use a questionable entry
/// costs a rebuild, whereas trusting one costs correctness.
///
/// # Arguments
///
/// * `output_dir` - The cached dictionary's output directory.
/// * `purpose` - What the build does with the dictionary.
/// * `expected` - The inputs of the current build ([`expected_build_inputs`]),
///   or `None` when this crate's source digest is unavailable.
/// * `source` - The metadata the crate ships.
///
/// # Returns
///
/// Why the entry may be used, or why not.
fn cache_decision(
    output_dir: &Path,
    purpose: CachePurpose,
    expected: Option<&BuildInputs>,
    source: &Metadata,
) -> Result<CacheHit, CacheMiss> {
    if !output_dir.is_dir() {
        return Err(CacheMiss::Empty);
    }
    if purpose == CachePurpose::Prefill {
        return Ok(CacheHit::Kept);
    }
    let expected = expected.ok_or(CacheMiss::DigestUnavailable)?;
    let data = fs::read(build_inputs_path(output_dir)).map_err(|_| CacheMiss::NoRecord)?;
    let recorded =
        serde_json::from_slice::<BuildInputs>(&data).map_err(|_| CacheMiss::UnreadableRecord)?;
    let changed = changed_inputs(&recorded, expected);
    if !changed.is_empty() {
        return Err(CacheMiss::InputsChanged(changed));
    }
    check_cached_metadata(output_dir, source)?;
    Ok(CacheHit::Current)
}

/// Whether a cached build's metadata equals the crate's, ignoring the fields
/// the build derives.
///
/// `DictionaryBuilder::build_metadata` writes the crate's metadata into the
/// output directory with three changes: it stamps `format_version` (which the
/// caller checks separately) and `lindera_version` (#1303), and, when
/// `connection_id_mapping` is on, embeds the computed `context_id_map`. A
/// cache built by another Lindera version is a miss already, as its
/// directory is named `<version>-fmt<format>` and `build_inputs.json`
/// records `builder_version`, so `lindera_version` needs no check of its
/// own. Everything else is copied verbatim, so any
/// other difference means the crate's `metadata.json` changed after the cache
/// was built. The comparison goes through the JSON representation so that
/// optional fields are treated identically on both sides and `Metadata` needs
/// no `PartialEq`.
///
/// # Arguments
///
/// * `cached` - The metadata read back from the cached build.
/// * `source` - The metadata the crate ships.
///
/// # Returns
///
/// `true` when the two agree on every field the build does not derive. A
/// serialization failure answers `false`, i.e. a cache miss.
fn metadata_matches_source(cached: &Metadata, source: &Metadata) -> bool {
    /// Fields `DictionaryBuilder::build_metadata` sets at build time.
    const BUILD_DERIVED_FIELDS: [&str; 3] = ["format_version", "lindera_version", "context_id_map"];

    let strip = |metadata: &Metadata| -> Option<serde_json::Value> {
        let mut value = serde_json::to_value(metadata).ok()?;
        if let Some(object) = value.as_object_mut() {
            for field in BUILD_DERIVED_FIELDS {
                object.remove(field);
            }
        }
        Some(value)
    };
    matches!((strip(cached), strip(source)), (Some(a), Some(b)) if a == b)
}

/// Returns the `cargo:` directives that decide when a dictionary crate's
/// build script reruns.
///
/// `context_id_freq.txt` is registered only when the crate ships it: cargo
/// treats a `rerun-if-changed` path that does not exist as always stale, so
/// registering it unconditionally reran the build script, and rebuilt the
/// dictionary, on every build of a crate without one (#1069). Adding the file
/// to such a crate later takes effect on the next change to a watched file
/// (such as `metadata.json`) or after a clean build.
///
/// The file that replaces it, named by `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE`
/// or its former name `LINDERA_CTX_FREQ_FILE`, is watched too, under the same
/// rule: only when it exists. Without that, a new histogram written to the
/// same path would not rerun the build script, and the dictionary would keep
/// the old ranking. Both variable names are watched, so that setting or
/// changing either reruns it.
///
/// # Arguments
///
/// * `ships_context_id_freq` - Whether the crate has a `context_id_freq.txt`.
/// * `ctx_freq_override` - The histogram chosen by
///   [`choose_context_id_freq_file`], if any.
///
/// # Returns
///
/// The directives, one per line, in the order the build script prints them.
fn rerun_directives(
    ships_context_id_freq: bool,
    ships_prebuilt: bool,
    ctx_freq_override: Option<&Path>,
) -> Vec<String> {
    let mut directives = vec![
        "cargo:rerun-if-changed=build.rs".to_string(),
        "cargo:rerun-if-changed=Cargo.toml".to_string(),
        // `metadata.json` drives build-time behavior (schema, flags such as
        // `connection_id_mapping`), so a change to it must rebuild the
        // dictionary.
        "cargo:rerun-if-changed=metadata.json".to_string(),
    ];
    if ships_context_id_freq {
        directives.push(format!("cargo:rerun-if-changed={CONTEXT_ID_FREQ_FILE}"));
    }
    // A new pin means another published dictionary, or the build from
    // source once its SHA-256 is cleared.
    if ships_prebuilt {
        directives.push(format!("cargo:rerun-if-changed={PREBUILT_FILE}"));
    }
    if let Some(path) = ctx_freq_override.filter(|path| path.is_file()) {
        directives.push(format!("cargo:rerun-if-changed={}", path.display()));
    }
    directives.extend([
        format!("cargo:rerun-if-env-changed={CONTEXT_ID_FREQ_FILE_ENV}"),
        format!("cargo:rerun-if-env-changed={DEPRECATED_CTX_FREQ_FILE_ENV}"),
        format!("cargo:rerun-if-env-changed={CACHE_DIR_ENV}"),
        format!("cargo:rerun-if-env-changed={DOCS_RS_ENV}"),
        format!("cargo:rerun-if-env-changed={FROM_SOURCE_ENV}"),
    ]);
    directives
}

/// Fetch the necessary assets and then build the dictionary using `builder`.
///
/// The dictionary is built under `LINDERA_BUILD_DICTIONARY_CACHE_DIR` when
/// that variable is set, reusing a cached build only when it was built from
/// the same inputs (see `cache_decision`), and under the build script's
/// `OUT_DIR` otherwise. With `DOCS_RS` set, it is a dummy built from
/// `params.dummy_input` instead of the downloaded one, and it always goes
/// under `OUT_DIR`, so that later builds never take a dummy from the cache.
///
/// Either way, the crate embeds the dictionary from `OUT_DIR`, which the
/// build script passes to it as `LINDERA_WORK_DIR`: a dictionary in the cache
/// is exported there, a hard link or a copy of each file, so that what the
/// crate embeds does not change when another build replaces the cache entry
/// or the cache is removed (#1343; see `fetch_into`).
///
/// Builds that share the cache write each dictionary one at a time: a build
/// that finds another build of the same dictionary writing it waits, and then
/// uses the dictionary that build left in place (see `fetch_into`).
///
/// With `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` set, the builder reads the
/// context-ID frequency histogram it names, in place of the one attached to
/// `builder`. `LINDERA_CTX_FREQ_FILE`, its former name, is read when it is
/// not set, until v8.0.0, with a `cargo:warning` that cargo shows only for a
/// path dependency.
///
/// # Arguments
///
/// * `params` - Describes the asset to fetch (archive name, mirrors, SHA-256)
///   and the input/output directory layout of the dictionary build.
/// * `builder` - Dictionary builder that turns the extracted MeCab sources into
///   the Lindera dictionary format.
///
/// # Returns
///
/// `Ok(())` once the dictionary has been built into the output directory, or a
/// `LinderaError` if the download, extraction, or build fails.
pub fn fetch(params: FetchParams, builder: DictionaryBuilder) -> LinderaResult<()> {
    fetch_for(params, builder, CachePurpose::Embed)
}

/// [`fetch`] for a build whose `purpose` decides how a cached dictionary is
/// treated.
///
/// # Arguments
///
/// * `params` - Describes the asset to fetch and the directory layout.
/// * `builder` - Dictionary builder.
/// * `purpose` - Whether the dictionary is embedded or only fills the cache.
///
/// # Returns
///
/// As for [`fetch`].
fn fetch_for(
    params: FetchParams,
    builder: DictionaryBuilder,
    purpose: CachePurpose,
) -> LinderaResult<()> {
    let ctx_freq = choose_context_id_freq_file(
        std::env::var_os(CONTEXT_ID_FREQ_FILE_ENV),
        std::env::var_os(DEPRECATED_CTX_FREQ_FILE_ENV),
    );
    if let Some(warning) = ctx_freq.warning {
        println!("cargo:warning={warning}");
    }
    for directive in rerun_directives(
        Path::new(CONTEXT_ID_FREQ_FILE).is_file(),
        Path::new(PREBUILT_FILE).is_file(),
        ctx_freq.path.as_deref(),
    ) {
        println!("{directive}");
    }
    let (from_source, from_source_warning) =
        from_source_requested(std::env::var_os(FROM_SOURCE_ENV));
    if let Some(warning) = from_source_warning {
        println!("cargo:warning={warning}");
    }
    let prebuilt = read_prebuilt(Path::new(PREBUILT_FILE))?;
    let source = choose_dictionary_source(
        is_docs_rs_build(),
        from_source,
        ctx_freq.path.is_some(),
        prebuilt.as_ref(),
        &builder.metadata().name,
    );
    if let DictionarySource::Source {
        reason: Some(reason),
    } = &source
    {
        println!("cargo:warning={reason}");
    }
    let builder = with_context_id_freq_override(builder, ctx_freq.path);

    // The directory the crate embeds the dictionary from, in every case.
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or_else(|| {
        LinderaErrorKind::Io.with_error(anyhow::anyhow!("OUT_DIR environment variable is not set"))
    })?); /* ex) target/debug/build/<pkg>/out */

    // Directory path for build package
    // if the cache directory variable is defined (and this is not a documentation build, see
    // `cache_dir_unless_docs_rs`), behaves like a cache, which a build reuses only when it was
    // built from the same inputs (see `cache_decision`); otherwise, keeps behavior of always
    // redownloading and rebuilding
    let (build_dir, is_cache) = if let Some(path) = dictionary_cache_dir_from_env() {
        let mut cache_dir = PathBuf::from(path);
        if !cache_dir.is_absolute()
            && let Ok(current_dir) = std::env::current_dir()
        {
            // If current_dir is a crate directory in a workspace, try to find the workspace root
            let mut root_dir = current_dir.clone();
            if let Some(parent) = current_dir.parent()
                && parent.join("Cargo.toml").exists()
            {
                root_dir = parent.to_path_buf();
            }
            cache_dir = root_dir.join(cache_dir);
        }

        let pkg_version = std::env::var("CARGO_PKG_VERSION").map_err(|_| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "CARGO_PKG_VERSION environment variable is not set"
            ))
        })?;

        // The dictionary format version is part of the cache key, not just the
        // crate version. Without it, bumping a dependency whose serialized
        // form is written verbatim -- crawdad for `dict.trie`, rkyv for
        // `char_def.bin`/`unk.bin` -- while reusing a cache directory would
        // serve artifacts in the old layout to a binary that walks a
        // different one. Keying on the format version turns that into a
        // cache miss.
        (
            cache_dir.join(format!("{pkg_version}-fmt{DICTIONARY_FORMAT_VERSION}")),
            true,
        )
    } else {
        (out_dir.clone(), false)
    };

    // The variable that `embedded_dictionary!` includes the dictionary from:
    // always `OUT_DIR`, never the cache, whose entry another build may
    // replace or a user may remove after this build (#1343).
    println!("cargo::rustc-env=LINDERA_WORK_DIR={}", out_dir.display());

    let hard_link = |from: &Path, to: &Path| fs::hard_link(from, to);
    let export_to = (is_cache && purpose == CachePurpose::Embed).then_some(ExportTo {
        dir: &out_dir,
        link: &hard_link,
    });
    fetch_into(
        &build_dir, is_cache, &source, &params, &builder, purpose, export_to,
    )
}

/// [`fetch_for`] once the build directory is known: fetches the source and
/// builds the dictionary into `build_dir`, unless the cache there already
/// holds it, and exports it to `export_to`.
///
/// In a cache, one build at a time writes a dictionary's entry, from the
/// cache check to the record of its inputs, under [`lock_cache_entry`]. The
/// entry is checked first without the lock, so a cache hit neither waits for
/// a build of the same dictionary nor needs a writable cache; on a miss, it
/// is checked again once the lock is held, so a build that waited uses the
/// dictionary the other build left instead of building it again.
///
/// A build that embeds a dictionary of the cache does not embed the entry's
/// files: it exports them to `export_to` (`OUT_DIR`), a hard link or a copy
/// of each file ([`export_entry`]), and the crate embeds the export (#1343).
/// Another build may then replace the entry, with other inputs, or the cache
/// may be removed, without changing what this build embeds. A hard link is
/// enough because an entry is replaced, never written in place: a build
/// writes the new dictionary into a temporary directory and moves it over
/// the emptied entry ([`empty_directory`], [`rename_directory`]), and the one
/// file written in place, `build_inputs.json`, is not exported. Nothing may
/// write an export in place either, as that would write the entry too.
///
/// On a hit, the entry is exported without the lock and checked again
/// afterwards. A build that replaced the entry meanwhile removed or changed
/// its record, so the second check misses, and this build goes on as on a
/// miss, with the lock, rather than embed the files of two entries. This
/// assumes that the entry is replaced at most once during the export: an
/// entry replaced twice, the second time with this build's inputs again,
/// passes the check even if the export took files of the one in between.
/// That takes two builds to finish within one export, a few milliseconds for
/// hard links. On a miss, the entry is exported while the lock is held;
/// where the lock cannot be taken, the entry is checked again after the
/// export, and a miss is an error.
///
/// # Arguments
///
/// * `build_dir` - The directory to build in: a cache directory, or `OUT_DIR`.
/// * `is_cache` - Whether `build_dir` is a cache, whose entries are reused.
/// * `source` - How to get the dictionary ([`choose_dictionary_source`]):
///   download the published one, build a dummy from `params.dummy_input`, or
///   build it from the source archive.
/// * `params` - Describes the asset to fetch and the directory layout.
/// * `builder` - Dictionary builder, with the histogram the build reads.
/// * `purpose` - Whether the dictionary is embedded or only fills the cache.
/// * `export_to` - Where to export the dictionary, for a build that embeds
///   it from the cache; `None` when the crate embeds it from `build_dir`
///   (`OUT_DIR`), or does not embed it.
///
/// # Returns
///
/// As for [`fetch`].
fn fetch_into(
    build_dir: &Path,
    is_cache: bool,
    source: &DictionarySource,
    params: &FetchParams,
    builder: &DictionaryBuilder,
    purpose: CachePurpose,
    export_to: Option<ExportTo<'_>>,
) -> LinderaResult<()> {
    fs::create_dir_all(build_dir).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to create build directory: {build_dir:?}"))
    })?;

    let input_dir = build_dir.join(params.input_dir);

    let output_dir = build_dir.join(params.output_dir);

    // Fast path where the data is already in cache.
    //
    // The directory name carries only the crate and format versions, which do
    // not change when the builder code, the source archive, the context-id
    // histogram or the crate's `metadata.json` does. `cache_decision` compares
    // the record written with the cached dictionary, and its `metadata.json`,
    // against the current build, and a cache entry that fails the check is
    // rebuilt rather than trusted.
    let expected = if is_cache {
        expected_build_inputs(params, builder, BUILDER_SOURCE_DIGEST, source)?
    } else {
        None
    };
    let decide = || cache_decision(&output_dir, purpose, expected.as_ref(), builder.metadata());
    let report_hit = |hit: CacheHit| {
        println!(
            "{CACHE_REPORT_PREFIX} using {}: {hit}",
            output_dir.display()
        );
    };
    // Exports the entry while the lock is held, or checks it again after the
    // export where it could not be taken.
    let export_held = |export_to: ExportTo<'_>, locked: bool| -> LinderaResult<()> {
        let dest = export_to.dir.join(params.output_dir);
        let export = export_entry(&output_dir, &dest, export_to.link)?;
        if !locked
            && expected.is_some()
            && let Err(miss) = decide()
        {
            return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "{} changed while it was exported to {} ({miss}), and it cannot be locked; \
                 build again",
                output_dir.display(),
                dest.display()
            )));
        }
        report_export(&output_dir, &dest, &export);
        Ok(())
    };
    // Bound to a name so that the lock is held until this function returns,
    // after the input directory is removed and the dictionary exported.
    let entry_lock = if is_cache {
        if expected.is_none() && purpose == CachePurpose::Embed {
            println!(
                "cargo:warning=the lindera-dictionary source digest is unavailable, \
                 so {} is rebuilt rather than reused",
                output_dir.display()
            );
        }
        if let Ok(hit) = decide() {
            let Some(export_to) = export_to else {
                report_hit(hit);
                return Ok(());
            };
            // Checked again after the export, without the lock: see above.
            let dest = export_to.dir.join(params.output_dir);
            if let Ok(export) = export_entry(&output_dir, &dest, export_to.link)
                && let Ok(hit) = decide()
            {
                report_hit(hit);
                report_export(&output_dir, &dest, &export);
                return Ok(());
            }
            println!(
                "{CACHE_REPORT_PREFIX} {} could not be exported, or changed while it was, so \
                 the build takes its lock",
                output_dir.display()
            );
        }
        let lock = lock_cache_entry(build_dir, params.output_dir)?;
        // Another build may have built the dictionary while this one waited.
        match decide() {
            Ok(hit) => {
                report_hit(hit);
                if let Some(export_to) = export_to {
                    export_held(export_to, lock.is_some())?;
                }
                return Ok(());
            }
            Err(miss) => println!(
                "{CACHE_REPORT_PREFIX} building {}: {miss}",
                output_dir.display()
            ),
        }
        lock
    } else {
        None
    };

    build_entry(
        build_dir,
        &input_dir,
        &output_dir,
        source,
        params,
        builder,
        expected.as_ref(),
    )?;
    if let Some(export_to) = export_to {
        export_held(export_to, entry_lock.is_some())?;
    }
    Ok(())
}

/// Builds or downloads the dictionary into `output_dir`, for [`fetch_into`],
/// once it knows that the cache, if any, does not hold it.
///
/// The dictionary is written into a temporary directory and replaces
/// `output_dir` ([`empty_directory`], [`rename_directory`]); in a cache, the
/// record of its inputs goes last, so that an interrupted build leaves no
/// record and the next build rebuilds.
///
/// # Arguments
///
/// * `build_dir` - The directory to build in: a cache directory, or `OUT_DIR`.
/// * `input_dir` - Where the source archive is extracted.
/// * `output_dir` - Where the dictionary goes.
/// * `source` - How to get the dictionary ([`choose_dictionary_source`]).
/// * `params` - Describes the asset to fetch and the directory layout.
/// * `builder` - Dictionary builder, with the histogram the build reads.
/// * `expected` - The inputs to record with the dictionary in a cache, or
///   `None` outside a cache or without this crate's source digest.
///
/// # Returns
///
/// `Ok(())` once the dictionary is in `output_dir`, or the error.
fn build_entry(
    build_dir: &Path,
    input_dir: &Path,
    output_dir: &Path,
    source: &DictionarySource,
    params: &FetchParams,
    builder: &DictionaryBuilder,
    expected: Option<&BuildInputs>,
) -> LinderaResult<()> {
    if let DictionarySource::Published { key, url, sha256 } = source {
        fetch_published(build_dir, output_dir, key, url, sha256, builder.metadata())?;
        // As below, the record goes last, once the dictionary is in place.
        if let Some(inputs) = &expected {
            write_build_inputs(output_dir, inputs)?;
        }
        return Ok(());
    }

    if *source == DictionarySource::Dummy {
        create_dummy_dictionary_source(input_dir, params.src_subdir, params.dummy_input)?;
    } else {
        // Source file path for build package
        let source_path_for_build = &build_dir.join(params.file_name);

        // Check if source file already exists and is valid
        let need_download = if source_path_for_build.exists() {
            debug!(
                "Found existing source file: {}",
                source_path_for_build.display()
            );

            // An archive left by an earlier build is used only with the pinned
            // SHA-256, as a download is (#1349).
            let actual_sha256 = file_sha256(source_path_for_build).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to read source file for SHA-256 check: {source_path_for_build:?}"
                    ))
            })?;

            if let Err(mismatch) = check_sha256(params.sha256, &actual_sha256) {
                warn!("{mismatch} for the cached file {source_path_for_build:?}");
                // Remove invalid file
                fs::remove_file(source_path_for_build).map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context(format!(
                            "Failed to remove invalid source file: {source_path_for_build:?}"
                        ))
                })?;
                true
            } else {
                debug!("SHA-256 check passed for cached file. Skipping download.");
                false
            }
        } else {
            debug!("Source file not found. Will download.");
            true
        };

        if need_download {
            // Download source file to build directory
            let tmp_download_path =
                Path::new(&build_dir).join(params.file_name.to_owned() + ".download");

            // Download a tarball. `http_status_as_error` is turned off so that a
            // non-2xx response is reported per URL and the next mirror is tried,
            // rather than aborting the round.
            let agent: Agent = Agent::config_builder()
                .user_agent(format!("Lindera/{}", env!("CARGO_PKG_VERSION")))
                .http_status_as_error(false)
                .build()
                .into();

            debug!("Downloading {:?}", params.download_urls);
            let mut dest = File::create(tmp_download_path.as_path()).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create temporary download file: {tmp_download_path:?}"
                    ))
            })?;
            let content = download_with_retry(
                &agent,
                params.download_urls.to_vec(),
                MAX_ROUND,
                params.sha256,
            )
            .map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!("{err}"))
                    .add_context("Failed to download dictionary assets")
            })?;

            io::copy(&mut Cursor::new(content.as_slice()), &mut dest).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to copy downloaded content to file: {tmp_download_path:?}"
                    ))
            })?;
            dest.flush().map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to flush download file: {tmp_download_path:?}"
                    ))
            })?;
            drop(dest);

            debug!("Content-Length: {}", content.len());
            debug!("Downloaded to {}", tmp_download_path.display());
            rename(tmp_download_path.clone(), source_path_for_build).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to rename temporary download file: {tmp_download_path:?} to {source_path_for_build:?}"
                    ))
            })?;

            info!("Source file cached at: {}", source_path_for_build.display());
        }

        // Decompress a tar.gz file
        let tmp_extract_path =
            Path::new(&build_dir).join(format!("tmp-archive-{}", params.input_dir));
        let tmp_extracted_path = tmp_extract_path.join(params.input_dir);
        let _ = fs::remove_dir_all(&tmp_extract_path);
        fs::create_dir_all(&tmp_extract_path).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to create temporary extraction directory: {tmp_extract_path:?}"
                ))
        })?;

        let mut tar_gz = File::open(source_path_for_build).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to open source file: {source_path_for_build:?}"
                ))
        })?;
        let mut buffer = Vec::new();
        tar_gz.read_to_end(&mut buffer).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to read source file: {source_path_for_build:?}"
                ))
        })?;
        let cursor = Cursor::new(buffer);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);
        archive.unpack(&tmp_extract_path).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to unpack archive: {source_path_for_build:?} to {tmp_extract_path:?}"
                ))
        })?;

        // Empty the input directory first to avoid conflicts when renaming the directory later on Linux and macOS systems (which do not support overwriting directories).
        empty_directory(input_dir)?;

        rename_directory(&tmp_extracted_path, input_dir)?;

        let _ = fs::remove_dir_all(&tmp_extract_path);
    }

    let tmp_output_path = build_dir.join(format!("tmp-output-{}", params.output_dir));
    let _ = fs::remove_dir_all(&tmp_output_path);

    let build_input_dir = match params.src_subdir {
        Some(subdir) => input_dir.join(subdir),
        None => input_dir.to_path_buf(),
    };

    builder
        .build_dictionary(&build_input_dir, &tmp_output_path)
        .map_err(|err| {
            LinderaErrorKind::Build
                .with_error(anyhow::anyhow!("{err}"))
                .add_context("Failed to build dictionary")
        })?;

    // Empty the output directory
    empty_directory(output_dir)?;

    // Rename tmp_output_path to output_dir
    rename_directory(&tmp_output_path, output_dir)?;

    // Record the inputs last, once the dictionary is in place, so that an
    // interrupted build leaves no record and the next build rebuilds.
    if let Some(inputs) = &expected {
        write_build_inputs(output_dir, inputs)?;
    }

    let _ = fs::remove_dir_all(input_dir);

    Ok(())
}

/// Where [`fetch_into`] exports the dictionary that a build embeds from the
/// cache, and how it puts each file there (#1343).
#[derive(Clone, Copy)]
struct ExportTo<'a> {
    /// The directory the crate embeds from (`OUT_DIR`); the dictionary goes
    /// into its sub-directory [`FetchParams::output_dir`].
    dir: &'a Path,
    /// Puts one file into place, from the entry's path to the export's:
    /// [`fs::hard_link`] in a build, a replacement in the tests.
    link: &'a dyn Fn(&Path, &Path) -> io::Result<()>,
}

/// How [`export_entry`] put a dictionary's files into place.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Export {
    /// Every file is a hard link to the entry's.
    Linked,
    /// At least one file could not be linked and was copied; the error of the
    /// first link that failed.
    Copied(String),
}

/// Puts the files of a dictionary that `embedded_dictionary!` includes
/// ([`DICTIONARY_FILES`]) from a cache entry into the directory that a crate
/// embeds them from (#1343).
///
/// `dest` is emptied first, then each file is put there with `link`, which is
/// [`fs::hard_link`] in a build, and copied where the link fails: across file
/// systems, or on one without hard links. A file that a failed link left
/// behind is removed before the copy, so that the copy never writes through
/// a link into the entry. The entry's other files, such as
/// `build_inputs.json`, are not exported.
///
/// # Arguments
///
/// * `entry` - The dictionary in the cache.
/// * `dest` - The directory to put its files into.
/// * `link` - Puts one file into place, from the entry's path to `dest`'s.
///
/// # Returns
///
/// How the files were put, or the error for a file that could be neither
/// linked nor copied.
fn export_entry(
    entry: &Path,
    dest: &Path,
    link: &dyn Fn(&Path, &Path) -> io::Result<()>,
) -> LinderaResult<Export> {
    empty_directory(dest)?;
    let mut first_link_error = None;
    for file in DICTIONARY_FILES {
        let from = entry.join(file);
        let to = dest.join(file);
        let Err(link_error) = link(&from, &to) else {
            continue;
        };
        let _ = fs::remove_file(&to);
        // One message rather than a context: `LinderaError` displays only the
        // outermost one, and both errors must show.
        fs::copy(&from, &to).map_err(|err| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "Failed to copy {from:?} to {to:?}: {err} (linking it failed: {link_error})"
            ))
        })?;
        first_link_error.get_or_insert_with(|| link_error.to_string());
    }
    Ok(match first_link_error {
        None => Export::Linked,
        Some(err) => Export::Copied(err),
    })
}

/// Prints the [`CACHE_REPORT_PREFIX`] line about an export, which
/// `cargo build -vv` shows.
///
/// # Arguments
///
/// * `entry` - The dictionary in the cache.
/// * `dest` - Where it was exported.
/// * `export` - How ([`export_entry`]).
fn report_export(entry: &Path, dest: &Path, export: &Export) {
    match export {
        Export::Linked => println!(
            "{CACHE_REPORT_PREFIX} linked {} into {}",
            entry.display(),
            dest.display()
        ),
        Export::Copied(err) => println!(
            "{CACHE_REPORT_PREFIX} copied {} into {}, as linking failed: {err}",
            entry.display(),
            dest.display()
        ),
    }
}

/// Downloads a published dictionary, checks it, and moves it into
/// `output_dir` (#1336).
///
/// Nothing reaches `output_dir` unless the archive has the pinned SHA-256 and
/// holds a directory named `key` with every file that `embedded_dictionary!`
/// includes and a `metadata.json` of this build's format version that
/// matches the crate's (apart from the fields the build derives). Any failure
/// stops the build: a dictionary that cannot be checked is never embedded,
/// and the build does not fall back to the source, so that a problem with the
/// published dictionaries shows instead of slowing every build down.
///
/// # Arguments
///
/// * `build_dir` - The build directory, for the temporary files.
/// * `output_dir` - Where the dictionary goes.
/// * `key` - The artifact's name, which is also the directory in the archive.
/// * `url` - Where to download the archive.
/// * `sha256` - The SHA-256 the archive must have.
/// * `source_metadata` - The metadata the crate ships.
///
/// # Returns
///
/// `Ok(())` once the dictionary is in `output_dir`, or the error.
fn fetch_published(
    build_dir: &Path,
    output_dir: &Path,
    key: &str,
    url: &str,
    sha256: &str,
    source_metadata: &Metadata,
) -> LinderaResult<()> {
    let hint = format!("set {FROM_SOURCE_ENV}=1 to build the dictionary from source instead");

    let agent: Agent = Agent::config_builder()
        .user_agent(format!("Lindera/{}", env!("CARGO_PKG_VERSION")))
        .http_status_as_error(false)
        .build()
        .into();
    debug!("Downloading the published dictionary {key} from {url}");
    // One message rather than a context: `LinderaError` displays only the
    // outermost one, and the reason must show.
    let content = download_with_retry(&agent, vec![url], MAX_ROUND, sha256).map_err(|err| {
        LinderaErrorKind::Io.with_error(anyhow::anyhow!(
            "Failed to download the published dictionary {key} from {url}: {err}; {hint}"
        ))
    })?;

    let tmp_extract_path = build_dir.join(format!("tmp-published-{key}"));
    let _ = fs::remove_dir_all(&tmp_extract_path);
    fs::create_dir_all(&tmp_extract_path).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to create temporary extraction directory: {tmp_extract_path:?}"
            ))
    })?;
    let result = unpack_published(&content, &tmp_extract_path, key, source_metadata, &hint)
        .and_then(|unpacked| {
            empty_directory(output_dir)?;
            rename_directory(&unpacked, output_dir)
        });
    let _ = fs::remove_dir_all(&tmp_extract_path);
    result?;
    info!("Using the published dictionary {key} from {url}");
    Ok(())
}

/// Unpacks a published dictionary's archive and checks what it holds.
///
/// # Arguments
///
/// * `content` - The archive, already checked against its SHA-256.
/// * `extract_dir` - An empty directory to unpack into.
/// * `key` - The artifact's name: the directory the archive must hold.
/// * `source_metadata` - The metadata the crate ships.
/// * `hint` - How to build the dictionary from source, for the errors.
///
/// # Returns
///
/// The unpacked dictionary directory, or an error naming what is wrong.
fn unpack_published(
    content: &[u8],
    extract_dir: &Path,
    key: &str,
    source_metadata: &Metadata,
    hint: &str,
) -> LinderaResult<PathBuf> {
    Archive::new(GzDecoder::new(Cursor::new(content)))
        .unpack(extract_dir)
        .map_err(|err| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "Failed to unpack the published dictionary {key}: {err}; {hint}"
            ))
        })?;
    let dictionary_dir = extract_dir.join(key);
    let missing: Vec<&str> = DICTIONARY_FILES
        .into_iter()
        .filter(|file| !dictionary_dir.join(file).is_file())
        .collect();
    if !missing.is_empty() {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "The published dictionary {key} has no {} in its directory {key}/; {hint}",
            missing.join(", ")
        )));
    }
    check_cached_metadata(&dictionary_dir, source_metadata).map_err(|mismatch| {
        LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "The published dictionary {key} does not fit this crate ({mismatch}): give the \
             dictionary a new revision in {PREBUILT_FILE} and clear its sha256 until it is \
             published, or {hint}"
        ))
    })?;
    Ok(dictionary_dir)
}

/// Name of the optional per-dictionary context-ID access-frequency histogram,
/// shipped in the dictionary crate root next to `metadata.json`. Produced by the
/// `ctxfreq` instrumentation (see the `ctxfreq_dump` example) and consumed when
/// `connection_id_mapping` is enabled, unless [`CONTEXT_ID_FREQ_FILE_ENV`]
/// names another file for the build.
const CONTEXT_ID_FREQ_FILE: &str = "context_id_freq.txt";

/// Shared body of every per-dictionary crate's `build.rs`.
///
/// Reads `metadata.json` from the crate root, fetches and builds the
/// dictionary described by `params`, and embeds the result under
/// `LINDERA_WORK_DIR`.
///
/// When the crate's embed feature is disabled (`embed_enabled == false`) and
/// no cache override is set via `LINDERA_BUILD_DICTIONARY_CACHE_DIR`, this is
/// a no-op so the crate builds without downloading any data. With the
/// override and the embed feature disabled, it only fills the cache: an
/// existing entry is kept without a check (`CachePurpose::Prefill`), so
/// that a workspace-wide `cargo check` does not rebuild every dictionary after
/// each change to the builder. With `DOCS_RS` set, the cache override is
/// ignored, so this is a no-op whenever the embed feature is disabled.
///
/// # Arguments
///
/// * `embed_enabled` - Whether the calling crate's embed feature is enabled,
///   i.e. whether the built dictionary is compiled into the binary.
/// * `params` - Describes the asset to fetch (archive name, mirrors, SHA-256)
///   and the input/output directory layout of the dictionary build.
///
/// # Returns
///
/// `Ok(())` once the dictionary has been built, or when the build was skipped
/// because neither the embed feature nor a cache override applies. Returns an
/// error if `metadata.json` cannot be read or the dictionary build fails.
pub fn build_embedded_dictionary(
    embed_enabled: bool,
    params: FetchParams,
) -> Result<(), Box<dyn Error>> {
    if dictionary_cache_dir_from_env().is_none() && !embed_enabled {
        return Ok(());
    }

    let metadata_json = fs::read_to_string("metadata.json")?;
    let metadata: crate::dictionary::metadata::Metadata = serde_json::from_str(&metadata_json)?;
    let mut builder = DictionaryBuilder::new(metadata);

    // A dictionary crate may ship a context-ID access-frequency histogram next to
    // `metadata.json`; it is what makes `connection_id_mapping` actually pay off.
    if std::path::Path::new(CONTEXT_ID_FREQ_FILE).is_file() {
        builder = builder.with_context_id_freq(CONTEXT_ID_FREQ_FILE);
    }

    let purpose = if embed_enabled {
        CachePurpose::Embed
    } else {
        CachePurpose::Prefill
    };
    fetch_for(params, builder, purpose)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    /// The directives every dictionary crate registers, in order.
    const ALWAYS: [&str; 8] = [
        "cargo:rerun-if-changed=build.rs",
        "cargo:rerun-if-changed=Cargo.toml",
        "cargo:rerun-if-changed=metadata.json",
        "cargo:rerun-if-env-changed=LINDERA_BUILD_CONTEXT_ID_FREQ_FILE",
        "cargo:rerun-if-env-changed=LINDERA_CTX_FREQ_FILE",
        "cargo:rerun-if-env-changed=LINDERA_BUILD_DICTIONARY_CACHE_DIR",
        "cargo:rerun-if-env-changed=DOCS_RS",
        "cargo:rerun-if-env-changed=LINDERA_BUILD_DICTIONARY_FROM_SOURCE",
    ];

    #[test]
    fn context_id_freq_file_takes_the_new_name() {
        assert_eq!(
            super::choose_context_id_freq_file(Some("new.txt".into()), None),
            super::ContextIdFreqFileChoice {
                path: Some("new.txt".into()),
                warning: None,
            }
        );
    }

    #[test]
    fn context_id_freq_file_takes_the_deprecated_name_with_a_warning() {
        assert_eq!(
            super::choose_context_id_freq_file(None, Some("old.txt".into())),
            super::ContextIdFreqFileChoice {
                path: Some("old.txt".into()),
                warning: Some(
                    "LINDERA_CTX_FREQ_FILE is deprecated and will be removed in v8.0.0; \
                     set LINDERA_BUILD_CONTEXT_ID_FREQ_FILE instead"
                ),
            }
        );
    }

    #[test]
    fn context_id_freq_file_prefers_the_new_name_and_warns_about_the_old_one() {
        assert_eq!(
            super::choose_context_id_freq_file(Some("new.txt".into()), Some("old.txt".into())),
            super::ContextIdFreqFileChoice {
                path: Some("new.txt".into()),
                warning: Some(
                    "LINDERA_CTX_FREQ_FILE is ignored because \
                     LINDERA_BUILD_CONTEXT_ID_FREQ_FILE is set; LINDERA_CTX_FREQ_FILE is \
                     deprecated and will be removed in v8.0.0"
                ),
            }
        );
    }

    #[test]
    fn context_id_freq_file_without_either_name() {
        assert_eq!(
            super::choose_context_id_freq_file(None, None),
            super::ContextIdFreqFileChoice {
                path: None,
                warning: None,
            }
        );
    }

    #[test]
    fn context_id_freq_override_replaces_the_bundled_histogram() {
        let builder = super::DictionaryBuilder::new(super::Metadata::default())
            .with_context_id_freq("context_id_freq.txt");
        let builder = super::with_context_id_freq_override(builder, Some("override.txt".into()));
        assert_eq!(
            builder.context_id_freq(),
            Some(std::path::Path::new("override.txt"))
        );
    }

    #[test]
    fn no_context_id_freq_override_keeps_the_bundled_histogram() {
        let builder = super::DictionaryBuilder::new(super::Metadata::default())
            .with_context_id_freq("context_id_freq.txt");
        let builder = super::with_context_id_freq_override(builder, None);
        assert_eq!(
            builder.context_id_freq(),
            Some(std::path::Path::new("context_id_freq.txt"))
        );
    }

    #[test]
    fn rerun_directives_skip_a_missing_context_id_freq_file() {
        // A missing `rerun-if-changed` path would rerun the build script,
        // and rebuild the dictionary, on every build (#1069).
        assert_eq!(super::rerun_directives(false, false, None), ALWAYS);
    }

    #[test]
    fn rerun_directives_watch_a_shipped_context_id_freq_file() {
        let mut expected = ALWAYS.to_vec();
        expected.insert(3, "cargo:rerun-if-changed=context_id_freq.txt");
        assert_eq!(super::rerun_directives(true, false, None), expected);
    }

    #[test]
    fn rerun_directives_watch_a_shipped_prebuilt_file() {
        // #1336: a new pin, or a cleared SHA-256, must rerun the build.
        let mut expected = ALWAYS.to_vec();
        expected.insert(3, "cargo:rerun-if-changed=prebuilt.json");
        assert_eq!(super::rerun_directives(false, true, None), expected);
    }

    #[test]
    fn rerun_directives_watch_an_existing_ctx_freq_override() {
        // A new histogram written to the same path must rerun the build.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("freq.txt");
        std::fs::write(&path, "").unwrap();
        let mut expected = ALWAYS.map(String::from).to_vec();
        expected.insert(3, format!("cargo:rerun-if-changed={}", path.display()));
        assert_eq!(super::rerun_directives(false, false, Some(&path)), expected);
    }

    #[test]
    fn rerun_directives_skip_a_missing_ctx_freq_override() {
        // Same rule as the shipped file (#1069): a missing path would rerun
        // the build script on every build.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.txt");
        assert_eq!(super::rerun_directives(false, false, Some(&path)), ALWAYS);
    }

    use super::*;
    use crate::dictionary::context_id_map::ContextIdMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_create_dummy_dictionary_source_without_subdir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_dir = temp_dir.path().join("mecab-test");

        create_dummy_dictionary_source(&input_dir, None, "dummy,0,0,0\n").unwrap();

        assert!(input_dir.join("char.def").is_file());
        assert!(input_dir.join("dummy_dict.csv").is_file());
        assert!(input_dir.join("unk.def").is_file());
        assert!(input_dir.join("matrix.def").is_file());
        assert_eq!(
            fs::read_to_string(input_dir.join("dummy_dict.csv")).unwrap(),
            "dummy,0,0,0\n"
        );
    }

    #[test]
    fn test_create_dummy_dictionary_source_with_subdir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_dir = temp_dir.path().join("mecab-test");

        create_dummy_dictionary_source(&input_dir, Some("dict-src"), "dummy,0,0,0\n").unwrap();

        // Files must be scaffolded where the dictionary builder reads from:
        // `input_dir/dict-src`, not `input_dir` itself.
        let src_dir = input_dir.join("dict-src");
        assert!(src_dir.join("char.def").is_file());
        assert!(src_dir.join("dummy_dict.csv").is_file());
        assert!(src_dir.join("unk.def").is_file());
        assert!(src_dir.join("matrix.def").is_file());
        assert!(!input_dir.join("char.def").exists());
    }

    #[test]
    fn test_create_dummy_dictionary_source_is_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_dir = temp_dir.path().join("mecab-test");

        // A second run against the same directory must succeed because docs.rs
        // reuses the build target directory across runs.
        create_dummy_dictionary_source(&input_dir, Some("dict-src"), "dummy,0,0,0\n").unwrap();
        create_dummy_dictionary_source(&input_dir, Some("dict-src"), "dummy,0,0,0\n").unwrap();

        assert!(input_dir.join("dict-src").join("char.def").is_file());
    }

    /// The dummy dictionary docs.rs builds must pass the dictionary build,
    /// which checks every entry's context ids against the dummy matrix. The
    /// entries have the shapes of the dictionary crates' `dummy_input`s.
    #[test]
    fn dummy_dictionary_source_builds() {
        for dummy_input in [
            "テスト,0,0,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
            "테스트,0,0,4721,NNG,행위,F,테스트,*,*,*,*\n",
            "测试,0,0,-1131,*,*,*,*,ce4 shi4,測試,测试,to test/\n",
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let input_dir = temp_dir.path().join("mecab-test");
            let output_dir = temp_dir.path().join("lindera-test");
            create_dummy_dictionary_source(&input_dir, Some("dict-src"), dummy_input).unwrap();

            DictionaryBuilder::new(Metadata::default())
                .build_dictionary(&input_dir.join("dict-src"), &output_dir)
                .unwrap_or_else(|err| panic!("the dummy for {dummy_input:?} must build: {err}"));
            let dictionary = crate::dictionary::Dictionary::load_from_path(&output_dir)
                .expect("the dummy dictionary loads");
            let surface = dummy_input.split(',').next().unwrap();
            assert_eq!(
                dictionary.prefix_dictionary.find_surface(surface).len(),
                1,
                "{surface}"
            );
        }
    }

    /// Writes `metadata` as `metadata.json` into a fresh directory, standing
    /// in for a cached dictionary build.
    fn cached_dir_with(metadata: &Metadata) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("metadata.json"),
            serde_json::to_vec_pretty(metadata).unwrap(),
        )
        .unwrap();
        dir
    }

    /// [`cached_dir_with`] for the default metadata declaring `format_version`.
    fn cached_dir_with_format_version(version: u32) -> tempfile::TempDir {
        cached_dir_with(&Metadata {
            format_version: version,
            ..Metadata::default()
        })
    }

    /// The metadata a crate would ship for the caches above: the default,
    /// whose `format_version` the build stamps over.
    fn source_metadata() -> Metadata {
        Metadata::default()
    }

    #[test]
    fn cache_hit_requires_the_current_format_version() {
        let dir = cached_dir_with_format_version(DICTIONARY_FORMAT_VERSION);
        assert_eq!(
            check_cached_metadata(dir.path(), &source_metadata()),
            Ok(())
        );
    }

    #[test]
    fn cache_from_another_format_version_is_a_miss() {
        // The directory name normally keys on the format version, so this
        // covers the leftovers: a hand-copied directory, or one from an
        // interrupted build.
        let dir = cached_dir_with_format_version(DICTIONARY_FORMAT_VERSION + 1);
        assert_eq!(
            check_cached_metadata(dir.path(), &source_metadata()),
            Err(CacheMiss::FormatVersion(DICTIONARY_FORMAT_VERSION + 1))
        );
    }

    #[test]
    fn cache_without_metadata_is_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            check_cached_metadata(dir.path(), &source_metadata()),
            Err(CacheMiss::UnreadableMetadata)
        );
    }

    #[test]
    fn cache_with_unparsable_metadata_is_a_miss() {
        // A half-written metadata.json from an interrupted build must not be
        // trusted; rebuilding is cheap, misreading a dictionary is not.
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("metadata.json"), b"{\"name\":").unwrap();
        assert_eq!(
            check_cached_metadata(dir.path(), &source_metadata()),
            Err(CacheMiss::UnreadableMetadata)
        );
    }

    #[test]
    fn cache_with_different_source_metadata_is_a_miss() {
        // The crate's metadata.json changed after the cache was built (here a
        // build-affecting flag) without a crate or format version bump: the
        // directory key still matches, so only the content comparison can
        // catch it.
        let dir = cached_dir_with_format_version(DICTIONARY_FORMAT_VERSION);
        let source = Metadata {
            connection_id_mapping: !Metadata::default().connection_id_mapping,
            ..Metadata::default()
        };
        assert_eq!(
            check_cached_metadata(dir.path(), &source),
            Err(CacheMiss::MetadataChanged)
        );
    }

    #[test]
    fn cache_ignores_build_derived_fields() {
        // The build stamps `format_version` and `lindera_version` and, with
        // connection-ID mapping, embeds the computed `context_id_map`; none
        // is in the crate's metadata.json, so none may count as a difference.
        let cached = Metadata {
            format_version: DICTIONARY_FORMAT_VERSION,
            lindera_version: Some(crate::get_version().to_string()),
            context_id_map: Some(ContextIdMap {
                left: vec![1, 0],
                right: vec![0, 1],
            }),
            ..Metadata::default()
        };
        let dir = cached_dir_with(&cached);
        assert_eq!(
            check_cached_metadata(dir.path(), &source_metadata()),
            Ok(())
        );
    }

    #[test]
    fn cache_hit_with_identical_non_default_source() {
        let source = Metadata {
            name: "ko-dic".to_string(),
            default_word_cost: -1,
            connection_id_mapping: true,
            ..Metadata::default()
        };
        let cached = Metadata {
            format_version: DICTIONARY_FORMAT_VERSION,
            ..source.clone()
        };
        let dir = cached_dir_with(&cached);
        assert_eq!(check_cached_metadata(dir.path(), &source), Ok(()));
    }

    /// The inputs of a build, standing in for [`expected_build_inputs`].
    fn build_inputs() -> BuildInputs {
        BuildInputs {
            builder_version: "7.0.0".to_string(),
            builder_digest: "0123456789abcdef0123456789abcdef".to_string(),
            archive_file_name: "mecab-ipadic-2.7.0-20250920.tar.gz".to_string(),
            archive_sha256: "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
                .to_string(),
            input_dir: "mecab-ipadic-2.7.0-20250920".to_string(),
            src_subdir: None,
            context_id_freq_md5: None,
            published: None,
        }
    }

    /// A cached dictionary of the current format version, built from
    /// `inputs` (no record when `None`).
    fn cached_entry(inputs: Option<&BuildInputs>) -> tempfile::TempDir {
        let dir = cached_dir_with_format_version(DICTIONARY_FORMAT_VERSION);
        if let Some(inputs) = inputs {
            write_build_inputs(dir.path(), inputs).unwrap();
        }
        dir
    }

    /// [`cache_decision`] for a dictionary that is embedded.
    fn decide(dir: &Path, expected: Option<&BuildInputs>) -> Result<CacheHit, CacheMiss> {
        cache_decision(dir, CachePurpose::Embed, expected, &source_metadata())
    }

    #[test]
    fn cache_hit_when_the_inputs_and_metadata_match() {
        let inputs = build_inputs();
        let dir = cached_entry(Some(&inputs));
        assert_eq!(decide(dir.path(), Some(&inputs)), Ok(CacheHit::Current));
    }

    #[test]
    fn each_changed_input_is_a_miss_naming_it() {
        // #1132: the builder code, the archive and the histogram decide the
        // output without changing the crate or format version.
        type Mutation = fn(&mut BuildInputs);
        let mutations: [(&str, Mutation); 7] = [
            ("builder_version", |i| {
                i.builder_version = "7.0.1".to_string()
            }),
            ("builder_digest", |i| {
                i.builder_digest.replace_range(0..1, "f")
            }),
            ("archive_file_name", |i| i.archive_file_name.push('x')),
            ("archive_sha256", |i| {
                i.archive_sha256.replace_range(0..1, "0")
            }),
            ("input_dir", |i| i.input_dir.push('x')),
            ("src_subdir", |i| {
                i.src_subdir = Some("dict-src".to_string())
            }),
            ("context_id_freq_md5", |i| {
                i.context_id_freq_md5 = Some("00".to_string())
            }),
        ];
        let recorded = build_inputs();
        let dir = cached_entry(Some(&recorded));
        for (field, mutate) in mutations {
            let mut expected = recorded.clone();
            mutate(&mut expected);
            assert_eq!(
                decide(dir.path(), Some(&expected)),
                Err(CacheMiss::InputsChanged(vec![field.to_string()])),
                "{field}"
            );
        }
    }

    #[test]
    fn cache_without_a_record_is_a_miss() {
        // A dictionary cached before the record existed, or by an
        // interrupted build.
        let dir = cached_entry(None);
        assert_eq!(
            decide(dir.path(), Some(&build_inputs())),
            Err(CacheMiss::NoRecord)
        );
    }

    #[test]
    fn cache_with_an_unparsable_record_is_a_miss() {
        let dir = cached_entry(None);
        fs::write(build_inputs_path(dir.path()), b"{\"builder_version\":").unwrap();
        assert_eq!(
            decide(dir.path(), Some(&build_inputs())),
            Err(CacheMiss::UnreadableRecord)
        );
    }

    #[test]
    fn cache_without_a_source_digest_is_a_miss() {
        // Without the digest nothing shows which builder made the entry.
        let inputs = build_inputs();
        let dir = cached_entry(Some(&inputs));
        assert_eq!(decide(dir.path(), None), Err(CacheMiss::DigestUnavailable));
    }

    #[test]
    fn matching_record_with_changed_metadata_is_a_miss() {
        let inputs = build_inputs();
        let dir = cached_entry(Some(&inputs));
        let source = Metadata {
            connection_id_mapping: !Metadata::default().connection_id_mapping,
            ..Metadata::default()
        };
        assert_eq!(
            cache_decision(dir.path(), CachePurpose::Embed, Some(&inputs), &source),
            Err(CacheMiss::MetadataChanged)
        );
    }

    #[test]
    fn rebuild_by_older_code_drops_the_record() {
        // A release from before the record existed rebuilds through
        // `empty_directory`; the record must go with the old output rather
        // than vouch for the new one.
        let inputs = build_inputs();
        let dir = cached_entry(Some(&inputs));
        let metadata = fs::read(dir.path().join("metadata.json")).unwrap();
        empty_directory(dir.path()).unwrap();
        fs::write(dir.path().join("metadata.json"), metadata).unwrap();
        assert_eq!(decide(dir.path(), Some(&inputs)), Err(CacheMiss::NoRecord));
    }

    #[test]
    fn missing_entry_is_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("lindera-ipadic");
        assert_eq!(
            decide(&output_dir, Some(&build_inputs())),
            Err(CacheMiss::Empty)
        );
        assert_eq!(
            cache_decision(&output_dir, CachePurpose::Prefill, None, &source_metadata()),
            Err(CacheMiss::Empty)
        );
    }

    #[test]
    fn prefill_keeps_an_existing_entry() {
        // With the embed feature off nothing is embedded, so a stale entry
        // is kept; rebuilding it would cost every workspace-wide check a
        // rebuild of every dictionary after each builder change.
        let dir = cached_entry(None);
        assert_eq!(
            cache_decision(
                dir.path(),
                CachePurpose::Prefill,
                Some(&build_inputs()),
                &source_metadata()
            ),
            Ok(CacheHit::Kept)
        );
    }

    /// Writes a crate `metadata.json` with `connection_id_mapping` and a
    /// histogram, and returns the builder and the histogram's path.
    fn builder_with_histogram(
        dir: &Path,
        connection_id_mapping: bool,
    ) -> (DictionaryBuilder, PathBuf) {
        let histogram = dir.join("context_id_freq.txt");
        fs::write(&histogram, "1 1 5\n").unwrap();
        let metadata = Metadata {
            connection_id_mapping,
            ..Metadata::default()
        };
        let builder = DictionaryBuilder::new(metadata).with_context_id_freq(&histogram);
        (builder, histogram)
    }

    /// The fetch parameters of a test build.
    fn fetch_params() -> FetchParams {
        FetchParams {
            file_name: "mecab-ipadic-2.7.0-20250920.tar.gz",
            input_dir: "mecab-ipadic-2.7.0-20250920",
            src_subdir: None,
            output_dir: "lindera-ipadic",
            dummy_input: "",
            download_urls: &[],
            sha256: "FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210",
        }
    }

    #[test]
    fn expected_inputs_follow_the_histogram_contents_with_mapping() {
        let dir = tempfile::tempdir().unwrap();
        let (builder, histogram) = builder_with_histogram(dir.path(), true);
        let before = expected_build_inputs(
            &fetch_params(),
            &builder,
            Some("d"),
            &DictionarySource::Source { reason: None },
        )
        .unwrap()
        .unwrap();
        fs::write(&histogram, "1 1 6\n").unwrap();
        let after = expected_build_inputs(
            &fetch_params(),
            &builder,
            Some("d"),
            &DictionarySource::Source { reason: None },
        )
        .unwrap()
        .unwrap();
        assert!(before.context_id_freq_md5.is_some());
        assert_ne!(before.context_id_freq_md5, after.context_id_freq_md5);
    }

    #[test]
    fn expected_inputs_follow_the_override_histogram() {
        // The build reads `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` in place of the
        // bundled file, so the record must hash that one.
        let dir = tempfile::tempdir().unwrap();
        let (builder, _) = builder_with_histogram(dir.path(), true);
        let override_path = dir.path().join("override.txt");
        fs::write(&override_path, "1 1 7\n").unwrap();
        let bundled = expected_build_inputs(
            &fetch_params(),
            &builder,
            Some("d"),
            &DictionarySource::Source { reason: None },
        )
        .unwrap()
        .unwrap();
        let builder = with_context_id_freq_override(builder, Some(override_path.clone()));
        let overridden = expected_build_inputs(
            &fetch_params(),
            &builder,
            Some("d"),
            &DictionarySource::Source { reason: None },
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            overridden.context_id_freq_md5,
            Some(file_md5(&override_path).unwrap())
        );
        assert_ne!(bundled.context_id_freq_md5, overridden.context_id_freq_md5);
    }

    #[test]
    fn expected_inputs_ignore_the_histogram_without_mapping() {
        // Without connection-ID mapping the build reads no histogram, so a
        // change to it must not cost a rebuild.
        let dir = tempfile::tempdir().unwrap();
        let (builder, _) = builder_with_histogram(dir.path(), false);
        let inputs = expected_build_inputs(
            &fetch_params(),
            &builder,
            Some("d"),
            &DictionarySource::Source { reason: None },
        )
        .unwrap()
        .unwrap();
        assert_eq!(inputs.context_id_freq_md5, None);
    }

    #[test]
    fn expected_inputs_describe_the_build() {
        let dir = tempfile::tempdir().unwrap();
        let (builder, _) = builder_with_histogram(dir.path(), false);
        let params = FetchParams {
            src_subdir: Some("dict-src"),
            ..fetch_params()
        };
        let inputs = expected_build_inputs(
            &params,
            &builder,
            Some("digest"),
            &DictionarySource::Source { reason: None },
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            inputs,
            BuildInputs {
                builder_version: env!("CARGO_PKG_VERSION").to_string(),
                builder_digest: "digest".to_string(),
                archive_file_name: params.file_name.to_string(),
                archive_sha256: "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
                    .to_string(),
                input_dir: params.input_dir.to_string(),
                src_subdir: Some("dict-src".to_string()),
                context_id_freq_md5: None,
                published: None,
            }
        );
    }

    #[test]
    fn no_expected_inputs_without_a_source_digest() {
        let dir = tempfile::tempdir().unwrap();
        let (builder, _) = builder_with_histogram(dir.path(), true);
        assert_eq!(
            expected_build_inputs(
                &fetch_params(),
                &builder,
                None,
                &DictionarySource::Source { reason: None }
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn this_build_has_a_source_digest() {
        // `build.rs` passes the digest under the `build_rs` feature, which
        // this module requires; without it the cache would never be reused.
        assert!(BUILDER_SOURCE_DIGEST.is_some_and(|digest| digest.len() == 32));
    }

    #[test]
    fn documentation_build_ignores_the_cache_dir() {
        // A dummy dictionary built into the cache would be embedded by later
        // builds without `DOCS_RS` (#1157).
        assert_eq!(
            cache_dir_unless_docs_rs(Some(OsString::from("/path/to/cache")), true),
            None
        );
    }

    #[test]
    fn regular_build_uses_the_cache_dir() {
        assert_eq!(
            cache_dir_unless_docs_rs(Some(OsString::from("/path/to/cache")), false),
            Some(OsString::from("/path/to/cache"))
        );
    }

    #[test]
    fn no_cache_dir_without_the_variable() {
        assert_eq!(cache_dir_unless_docs_rs(None, false), None);
        assert_eq!(cache_dir_unless_docs_rs(None, true), None);
    }

    /// Directory the dummy archive of the cache tests extracts to.
    const SEEDED_INPUT_DIR: &str = "mecab-test";

    /// Directory of the dictionary the cache tests build.
    const SEEDED_OUTPUT_DIR: &str = "lindera-test";

    /// File name of the dummy archive.
    const SEEDED_ARCHIVE: &str = "mecab-test.tar.gz";

    /// The one entry of the dummy dictionary.
    const SEEDED_LEXICON: &str = "テスト,0,0,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n";

    /// A cache directory holding the source archive of a dummy dictionary,
    /// as a download leaves it, so that a build needs no network; and the
    /// archive's SHA-256, for [`seeded_params`].
    fn seeded_cache() -> (tempfile::TempDir, &'static str) {
        let cache = tempfile::tempdir().unwrap();
        let sha256 = write_source_archive(cache.path(), SEEDED_ARCHIVE, SEEDED_LEXICON);
        (cache, sha256)
    }

    /// Writes the source archive of a dummy dictionary with `lexicon` into
    /// `cache` as `name`, as a download leaves it, extracting to
    /// [`SEEDED_INPUT_DIR`]; returns its SHA-256.
    fn write_source_archive(cache: &Path, name: &str, lexicon: &str) -> &'static str {
        let sources = tempfile::tempdir().unwrap();
        let source_dir = sources.path().join(SEEDED_INPUT_DIR);
        create_dummy_dictionary_source(&source_dir, None, lexicon).unwrap();
        let archive_path = cache.join(name);
        let encoder = flate2::write::GzEncoder::new(
            File::create(&archive_path).unwrap(),
            flate2::Compression::fast(),
        );
        let mut archive = tar::Builder::new(encoder);
        archive
            .append_dir_all(SEEDED_INPUT_DIR, &source_dir)
            .unwrap();
        archive.into_inner().unwrap().finish().unwrap();
        let sha256 = file_sha256(&archive_path).unwrap();
        Box::leak(sha256.into_boxed_str())
    }

    /// The parameters that build the dummy dictionary of [`seeded_cache`].
    fn seeded_params(sha256: &'static str) -> FetchParams {
        FetchParams {
            file_name: SEEDED_ARCHIVE,
            input_dir: SEEDED_INPUT_DIR,
            src_subdir: None,
            output_dir: SEEDED_OUTPUT_DIR,
            dummy_input: "",
            download_urls: &[],
            sha256,
        }
    }

    /// Builds the dummy dictionary into the cache `build_dir`, to embed it.
    fn build_seeded(build_dir: &Path, sha256: &'static str) -> LinderaResult<()> {
        build_with(build_dir, &seeded_params(sha256), None)
    }

    /// Builds the dummy dictionary of `params`' archive into the cache
    /// `build_dir`, to embed it, exporting it to `export_to`.
    fn build_with(
        build_dir: &Path,
        params: &FetchParams,
        export_to: Option<ExportTo<'_>>,
    ) -> LinderaResult<()> {
        fetch_into(
            build_dir,
            true,
            &DictionarySource::Source { reason: None },
            params,
            &DictionaryBuilder::new(source_metadata()),
            CachePurpose::Embed,
            export_to,
        )
    }

    /// The record a build of the dummy dictionary writes.
    fn seeded_inputs(sha256: &'static str) -> BuildInputs {
        expected_build_inputs(
            &seeded_params(sha256),
            &DictionaryBuilder::new(source_metadata()),
            BUILDER_SOURCE_DIGEST,
            &DictionarySource::Source { reason: None },
        )
        .unwrap()
        .unwrap()
    }

    /// Writes a current entry for the dummy dictionary, standing in for the
    /// one another build leaves, with a marker file that a rebuild removes;
    /// returns the marker's path.
    fn write_current_entry(build_dir: &Path, sha256: &'static str) -> PathBuf {
        let output_dir = build_dir.join(SEEDED_OUTPUT_DIR);
        fs::create_dir_all(&output_dir).unwrap();
        let metadata = Metadata {
            format_version: DICTIONARY_FORMAT_VERSION,
            ..source_metadata()
        };
        fs::write(
            output_dir.join("metadata.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
        write_build_inputs(&output_dir, &seeded_inputs(sha256)).unwrap();
        let marker = output_dir.join("marker");
        fs::write(&marker, b"").unwrap();
        marker
    }

    /// Takes the lock on the dummy dictionary's entry, as another build does.
    fn hold_entry_lock(build_dir: &Path) -> File {
        lock_cache_entry(build_dir, SEEDED_OUTPUT_DIR)
            .unwrap()
            .expect("this platform locks files")
    }

    #[test]
    fn a_waiting_build_uses_the_entry_built_meanwhile() {
        // #1206: a build that finds another build of the same dictionary
        // writing it waits, and then uses what that build left.
        let (cache, sha256) = seeded_cache();
        let build_dir = cache.path().to_path_buf();
        let lock = hold_entry_lock(&build_dir);
        let waiting = {
            let build_dir = build_dir.clone();
            std::thread::spawn(move || build_seeded(&build_dir, sha256))
        };
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            !waiting.is_finished(),
            "the build did not wait for the lock"
        );
        for name in [
            format!("tmp-archive-{SEEDED_INPUT_DIR}"),
            SEEDED_INPUT_DIR.to_string(),
            format!("tmp-output-{SEEDED_OUTPUT_DIR}"),
            SEEDED_OUTPUT_DIR.to_string(),
        ] {
            assert!(
                !build_dir.join(&name).exists(),
                "{name} written without the lock"
            );
        }
        let marker = write_current_entry(&build_dir, sha256);
        drop(lock);
        waiting.join().unwrap().unwrap();
        assert!(marker.exists(), "the entry was built again");
    }

    #[test]
    fn a_cache_hit_does_not_wait_for_the_lock() {
        // A current entry is used without the lock, so that a read-only
        // cache works and builds that only read it do not queue up.
        let (cache, sha256) = seeded_cache();
        let build_dir = cache.path().to_path_buf();
        let marker = write_current_entry(&build_dir, sha256);
        let _lock = hold_entry_lock(&build_dir);
        let build = {
            let build_dir = build_dir.clone();
            std::thread::spawn(move || build_seeded(&build_dir, sha256))
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !build.is_finished() {
            assert!(
                std::time::Instant::now() < deadline,
                "a cache hit waited for the lock"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        build.join().unwrap().unwrap();
        assert!(marker.exists(), "the entry was built again");
    }

    #[test]
    fn concurrent_builds_leave_one_complete_entry() {
        // #1206: builds sharing the cache removed each other's files, so
        // some failed and the cache could end without the dictionary.
        const BUILDS: usize = 8;
        const ROUNDS: usize = 10;
        for round in 0..ROUNDS {
            let (cache, sha256) = seeded_cache();
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(BUILDS));
            let builds: Vec<_> = (0..BUILDS)
                .map(|_| {
                    let build_dir = cache.path().to_path_buf();
                    let barrier = barrier.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        build_seeded(&build_dir, sha256)
                    })
                })
                .collect();
            for build in builds {
                build
                    .join()
                    .unwrap()
                    .unwrap_or_else(|err| panic!("round {round}: {err}"));
            }

            let output_dir = cache.path().join(SEEDED_OUTPUT_DIR);
            assert_eq!(
                decide(&output_dir, Some(&seeded_inputs(sha256))),
                Ok(CacheHit::Current),
                "round {round}"
            );
            let dictionary = crate::dictionary::Dictionary::load_from_path(&output_dir)
                .unwrap_or_else(|err| panic!("round {round}: {err}"));
            assert_eq!(
                dictionary.prefix_dictionary.find_surface("テスト").len(),
                1,
                "round {round}"
            );
            drop(dictionary);
            let mut left: Vec<_> = fs::read_dir(cache.path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect();
            left.sort();
            assert_eq!(
                left,
                [
                    SEEDED_OUTPUT_DIR.to_string(),
                    format!("{SEEDED_OUTPUT_DIR}{CACHE_LOCK_SUFFIX}"),
                    SEEDED_ARCHIVE.to_string(),
                ],
                "round {round}"
            );
        }
    }

    // The published dictionaries (#1336).

    /// The SHA-256 of `abc`.
    const SHA256_OF_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    /// The key of the test's published dictionary.
    const PUBLISHED_KEY: &str = "test-1.0-fmt3-r1";

    /// A pin of the test's published dictionary.
    fn pin(format_version: u32, sha256: Option<&str>) -> PrebuiltDictionary {
        PrebuiltDictionary {
            key: PUBLISHED_KEY.to_string(),
            format_version,
            base_url: "https://example.com/releases/download/".to_string(),
            path: "/dictionary-test-1.0-fmt3-r1/test-1.0-fmt3-r1.tar.gz".to_string(),
            sha256: sha256.map(str::to_string),
        }
    }

    /// The reason of a build from source, panicking on any other choice.
    fn source_reason(source: DictionarySource) -> String {
        match source {
            DictionarySource::Source {
                reason: Some(reason),
            } => reason,
            other => panic!("expected a build from source with a reason, got {other:?}"),
        }
    }

    #[test]
    fn a_pinned_dictionary_is_used_by_default() {
        let pinned = pin(DICTIONARY_FORMAT_VERSION, Some(SHA256_OF_ABC));
        assert_eq!(
            choose_dictionary_source(false, false, false, Some(&pinned), "test"),
            DictionarySource::Published {
                key: PUBLISHED_KEY.to_string(),
                url: "https://example.com/releases/download/dictionary-test-1.0-fmt3-r1/\
                      test-1.0-fmt3-r1.tar.gz"
                    .to_string(),
                sha256: SHA256_OF_ABC.to_string(),
            }
        );
    }

    #[test]
    fn docs_rs_builds_a_dummy_even_with_a_pin() {
        let pinned = pin(DICTIONARY_FORMAT_VERSION, Some(SHA256_OF_ABC));
        assert_eq!(
            choose_dictionary_source(true, false, false, Some(&pinned), "test"),
            DictionarySource::Dummy
        );
    }

    #[test]
    fn without_a_pinned_sha256_the_build_is_from_source_without_a_warning() {
        let unpublished = pin(DICTIONARY_FORMAT_VERSION, None);
        for (prebuilt, from_source, context_id_freq_override) in [
            (None, false, false),
            (Some(&unpublished), false, false),
            (None, true, true),
            (Some(&unpublished), true, true),
        ] {
            assert_eq!(
                choose_dictionary_source(
                    false,
                    from_source,
                    context_id_freq_override,
                    prebuilt,
                    "test"
                ),
                DictionarySource::Source { reason: None },
                "{prebuilt:?}, {from_source}, {context_id_freq_override}"
            );
        }
    }

    #[test]
    fn forcing_the_build_from_source_says_so() {
        let pinned = pin(DICTIONARY_FORMAT_VERSION, Some(SHA256_OF_ABC));
        let reason = source_reason(choose_dictionary_source(
            false,
            true,
            false,
            Some(&pinned),
            "test",
        ));
        assert!(reason.contains(FROM_SOURCE_ENV), "{reason}");
    }

    #[test]
    fn another_context_id_histogram_builds_from_source() {
        let pinned = pin(DICTIONARY_FORMAT_VERSION, Some(SHA256_OF_ABC));
        let reason = source_reason(choose_dictionary_source(
            false,
            false,
            true,
            Some(&pinned),
            "test",
        ));
        assert!(reason.contains(CONTEXT_ID_FREQ_FILE_ENV), "{reason}");
    }

    #[test]
    fn another_format_version_builds_from_source() {
        let pinned = pin(DICTIONARY_FORMAT_VERSION + 1, Some(SHA256_OF_ABC));
        let reason = source_reason(choose_dictionary_source(
            false,
            false,
            false,
            Some(&pinned),
            "test",
        ));
        assert!(
            reason.contains(&format!(
                "{PUBLISHED_KEY} has format version {}, and this build reads format version \
                 {DICTIONARY_FORMAT_VERSION}",
                DICTIONARY_FORMAT_VERSION + 1
            )),
            "{reason}"
        );
    }

    #[test]
    fn the_build_from_source_is_forced_by_1_and_true() {
        for (value, expected) in [
            (None, false),
            (Some("1"), true),
            (Some("true"), true),
            (Some("TRUE"), true),
            (Some(""), false),
            (Some("0"), false),
            (Some("false"), false),
        ] {
            assert_eq!(
                from_source_requested(value.map(OsString::from)),
                (expected, None),
                "{value:?}"
            );
        }
        let (forced, warning) = from_source_requested(Some(OsString::from("yes")));
        assert!(forced);
        assert!(
            warning
                .as_deref()
                .is_some_and(|warning| warning.contains("=yes")),
            "{warning:?}"
        );
    }

    #[test]
    fn a_pin_is_read_and_a_missing_one_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(PREBUILT_FILE);
        assert_eq!(read_prebuilt(&path).unwrap(), None);
        fs::write(
            &path,
            r#"{"key": "k", "format_version": 3, "base_url": "https://b", "path": "p.tar.gz", "sha256": null}"#,
        )
        .unwrap();
        assert_eq!(
            read_prebuilt(&path).unwrap(),
            Some(PrebuiltDictionary {
                key: "k".to_string(),
                format_version: 3,
                base_url: "https://b".to_string(),
                path: "p.tar.gz".to_string(),
                sha256: None,
            })
        );
        // A misspelled field must not pass as a pin without a SHA-256.
        fs::write(
            &path,
            r#"{"key": "k", "format_version": 3, "base_url": "https://b", "path": "p", "sha_256": "x"}"#,
        )
        .unwrap();
        assert!(read_prebuilt(&path).is_err());
    }

    #[test]
    fn sha256_digests_are_compared_ignoring_case() {
        assert_eq!(sha256_hex(b"abc"), SHA256_OF_ABC);
        assert_eq!(check_sha256(SHA256_OF_ABC, &sha256_hex(b"abc")), Ok(()));
        assert_eq!(
            check_sha256(&SHA256_OF_ABC.to_ascii_uppercase(), &sha256_hex(b"abc")),
            Ok(())
        );
        let mismatch = check_sha256(SHA256_OF_ABC, &sha256_hex(b"abd")).unwrap_err();
        assert_eq!(
            mismatch,
            format!(
                "SHA-256 mismatch: expected {SHA256_OF_ABC}, got {}",
                sha256_hex(b"abd")
            )
        );
    }

    #[test]
    fn a_file_is_hashed_as_its_content() {
        // Longer than the 8192-byte buffer, so that the file is read in
        // several chunks.
        let content: Vec<u8> = (0..20_000u32).map(|i| (i % 251) as u8).collect();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("archive.tar.gz");
        fs::write(&path, &content).unwrap();
        assert_eq!(file_sha256(&path).unwrap(), sha256_hex(&content));
    }

    /// [`seeded_params`] with `sha256`, downloading the archive from `url`
    /// alone.
    fn params_downloading_from(sha256: &'static str, url: String) -> FetchParams {
        let urls: &'static [&'static str] =
            Box::leak(vec![&*Box::leak(url.into_boxed_str())].into_boxed_slice());
        FetchParams {
            download_urls: urls,
            ..seeded_params(sha256)
        }
    }

    #[test]
    fn a_downloaded_source_archive_with_another_sha256_stops_the_build() {
        // The crate pins the seeded archive, and the host serves another one.
        let archives = tempfile::tempdir().unwrap();
        let pinned = write_source_archive(archives.path(), SEEDED_ARCHIVE, SEEDED_LEXICON);
        add_other_archive(archives.path());
        let (base, requests) = serve(200, fs::read(archives.path().join(OTHER_ARCHIVE)).unwrap());
        let params = params_downloading_from(pinned, format!("{base}/{SEEDED_ARCHIVE}"));
        let cache = tempfile::tempdir().unwrap();

        let failure = failure(build_with(cache.path(), &params, None));
        assert!(
            failure.contains(&format!("SHA-256 mismatch: expected {pinned}")),
            "{failure}"
        );
        assert_eq!(requests.load(Ordering::SeqCst), MAX_ROUND);
        assert!(!cache.path().join(SEEDED_ARCHIVE).exists());
        assert!(!cache.path().join(SEEDED_OUTPUT_DIR).exists());
    }

    #[test]
    fn a_cached_source_archive_with_another_sha256_is_downloaded_again() {
        let archives = tempfile::tempdir().unwrap();
        let pinned = write_source_archive(archives.path(), SEEDED_ARCHIVE, SEEDED_LEXICON);
        let (base, requests) = serve(200, fs::read(archives.path().join(SEEDED_ARCHIVE)).unwrap());
        let params = params_downloading_from(pinned, format!("{base}/{SEEDED_ARCHIVE}"));
        // The build directory holds another archive under the pinned name, as
        // a download from a host that served it would have left it.
        let cache = tempfile::tempdir().unwrap();
        write_source_archive(cache.path(), SEEDED_ARCHIVE, OTHER_LEXICON);

        build_with(cache.path(), &params, None).unwrap();
        assert_eq!(requests.load(Ordering::SeqCst), 1);
        assert_eq!(
            file_sha256(&cache.path().join(SEEDED_ARCHIVE)).unwrap(),
            pinned
        );
        // The dictionary is the one of the pinned archive's entry.
        let (reference, reference_sha256) = seeded_cache();
        build_seeded(reference.path(), reference_sha256).unwrap();
        assert_eq!(
            dictionary_bytes(&cache.path().join(SEEDED_OUTPUT_DIR)),
            dictionary_bytes(&reference.path().join(SEEDED_OUTPUT_DIR))
        );
    }

    #[test]
    fn a_cached_source_archive_is_used_with_a_pin_in_upper_case() {
        let (cache, sha256) = seeded_cache();
        let upper: &'static str = Box::leak(sha256.to_ascii_uppercase().into_boxed_str());
        // `seeded_params` has no URLs, so the build fails if it downloads.
        build_with(cache.path(), &seeded_params(upper), None).unwrap();

        // The record keeps the digest in lowercase, so that the case of the
        // pin does not decide whether the entry is current.
        let output = cache.path().join(SEEDED_OUTPUT_DIR);
        let record: BuildInputs =
            serde_json::from_slice(&fs::read(build_inputs_path(&output)).unwrap()).unwrap();
        assert_eq!(record.archive_sha256, sha256);
    }

    #[test]
    fn a_record_with_the_md5_of_the_archive_is_a_miss() {
        // A record written before #1349 names the MD5 of the archive.
        let inputs = build_inputs();
        let dir = cached_entry(Some(&inputs));
        let path = build_inputs_path(dir.path());
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let fields = record.as_object_mut().unwrap();
        assert!(fields.remove("archive_sha256").is_some());
        fields.insert(
            "archive_md5".to_string(),
            "fedcba9876543210fedcba9876543210".into(),
        );
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();

        assert_eq!(
            decide(dir.path(), Some(&inputs)),
            Err(CacheMiss::UnreadableRecord)
        );
    }

    #[test]
    fn a_record_from_before_the_published_field_reads_as_built_from_source() {
        let mut record = serde_json::to_value(build_inputs()).unwrap();
        record.as_object_mut().unwrap().remove("published");
        let record: BuildInputs = serde_json::from_value(record).unwrap();
        assert_eq!(record.published, None);
        assert!(changed_inputs(&record, &build_inputs()).is_empty());
    }

    /// Serves `body` with `status` to every request on 127.0.0.1.
    ///
    /// # Returns
    ///
    /// The server's base URL and the number of requests it has answered.
    fn serve(status: u16, body: Vec<u8>) -> (String, Arc<AtomicUsize>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let answered = Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                // Read the request's head before answering.
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    match stream.read(&mut byte) {
                        Ok(1) => head.push(byte[0]),
                        _ => break,
                    }
                }
                answered.fetch_add(1, Ordering::SeqCst);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} Status\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(&body);
            }
        });
        (format!("http://{address}"), requests)
    }

    /// Builds the dummy dictionary from source and packs `files` of it as a
    /// published dictionary is packed: in one directory named
    /// [`PUBLISHED_KEY`].
    ///
    /// # Returns
    ///
    /// The archive, and the cache that holds the dictionary built from
    /// source, in [`SEEDED_OUTPUT_DIR`].
    fn published_archive(files: &[&str]) -> (Vec<u8>, tempfile::TempDir) {
        let (cache, sha256) = seeded_cache();
        build_seeded(cache.path(), sha256).unwrap();
        let archive = pack_published(&cache.path().join(SEEDED_OUTPUT_DIR), files);
        (archive, cache)
    }

    /// Packs `files` of the dictionary in `built` as a published dictionary
    /// is packed: in one directory named [`PUBLISHED_KEY`].
    fn pack_published(built: &Path, files: &[&str]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut archive = tar::Builder::new(encoder);
        for file in files {
            archive
                .append_path_with_name(built.join(file), format!("{PUBLISHED_KEY}/{file}"))
                .unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap()
    }

    /// Gets the published dictionary at `url` into the cache `build_dir`, for
    /// a crate that ships `metadata`.
    fn fetch_published_into(
        build_dir: &Path,
        url: &str,
        sha256: &str,
        metadata: Metadata,
    ) -> LinderaResult<()> {
        fetch_published_with(build_dir, url, sha256, metadata, None)
    }

    /// [`fetch_published_into`], exporting the dictionary to `export_to`.
    fn fetch_published_with(
        build_dir: &Path,
        url: &str,
        sha256: &str,
        metadata: Metadata,
        export_to: Option<ExportTo<'_>>,
    ) -> LinderaResult<()> {
        fetch_into(
            build_dir,
            true,
            &DictionarySource::Published {
                key: PUBLISHED_KEY.to_string(),
                url: url.to_string(),
                sha256: sha256.to_string(),
            },
            &seeded_params("unused"),
            &DictionaryBuilder::new(metadata),
            CachePurpose::Embed,
            export_to,
        )
    }

    /// The text of a failed fetch, with its whole chain of causes.
    fn failure(result: LinderaResult<()>) -> String {
        format!("{:?}", result.expect_err("the fetch must fail"))
    }

    #[test]
    fn a_published_dictionary_is_downloaded_checked_and_cached() {
        let (archive, built) = published_archive(&DICTIONARY_FILES);
        let sha256 = sha256_hex(&archive);
        let (base, requests) = serve(200, archive);
        let url = format!("{base}/{PUBLISHED_KEY}.tar.gz");
        let cache = tempfile::tempdir().unwrap();
        fetch_published_into(cache.path(), &url, &sha256, source_metadata()).unwrap();

        let output = cache.path().join(SEEDED_OUTPUT_DIR);
        for file in DICTIONARY_FILES {
            assert_eq!(
                fs::read(output.join(file)).unwrap(),
                fs::read(built.path().join(SEEDED_OUTPUT_DIR).join(file)).unwrap(),
                "{file}"
            );
        }
        crate::dictionary::Dictionary::load_from_path(&output).unwrap();
        let record: BuildInputs =
            serde_json::from_slice(&fs::read(build_inputs_path(&output)).unwrap()).unwrap();
        assert_eq!(
            record.published,
            Some(PublishedRecord {
                key: PUBLISHED_KEY.to_string(),
                sha256: sha256.clone(),
            })
        );
        assert_eq!(requests.load(Ordering::SeqCst), 1);

        // The next build uses the cached entry without downloading it again.
        fetch_published_into(cache.path(), &url, &sha256, source_metadata()).unwrap();
        assert_eq!(requests.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_published_dictionary_with_another_sha256_stops_the_build() {
        let (archive, _built) = published_archive(&DICTIONARY_FILES);
        let (base, requests) = serve(200, archive);
        let url = format!("{base}/{PUBLISHED_KEY}.tar.gz");
        let cache = tempfile::tempdir().unwrap();
        let other = "0".repeat(64);
        let failure = failure(fetch_published_into(
            cache.path(),
            &url,
            &other,
            source_metadata(),
        ));
        assert!(
            failure.contains(&format!("SHA-256 mismatch: expected {other}")),
            "{failure}"
        );
        assert!(failure.contains(FROM_SOURCE_ENV), "{failure}");
        assert!(!cache.path().join(SEEDED_OUTPUT_DIR).exists());
        assert_eq!(requests.load(Ordering::SeqCst), MAX_ROUND);
    }

    #[test]
    fn a_failed_download_stops_the_build() {
        let (base, _requests) = serve(404, b"Not Found".to_vec());
        let url = format!("{base}/{PUBLISHED_KEY}.tar.gz");
        let cache = tempfile::tempdir().unwrap();
        let failure = failure(fetch_published_into(
            cache.path(),
            &url,
            SHA256_OF_ABC,
            source_metadata(),
        ));
        assert!(failure.contains("HTTP 404"), "{failure}");
        assert!(failure.contains(&url), "{failure}");
        assert!(!cache.path().join(SEEDED_OUTPUT_DIR).exists());
    }

    #[test]
    fn a_published_dictionary_without_a_file_stops_the_build() {
        let (archive, _built) = published_archive(&DICTIONARY_FILES[..8]);
        let sha256 = sha256_hex(&archive);
        let (base, _requests) = serve(200, archive);
        let url = format!("{base}/{PUBLISHED_KEY}.tar.gz");
        let cache = tempfile::tempdir().unwrap();
        let failure = failure(fetch_published_into(
            cache.path(),
            &url,
            &sha256,
            source_metadata(),
        ));
        assert!(failure.contains("has no metadata.json"), "{failure}");
        assert!(!cache.path().join(SEEDED_OUTPUT_DIR).exists());
    }

    #[test]
    fn a_published_dictionary_of_another_crate_stops_the_build() {
        let (archive, _built) = published_archive(&DICTIONARY_FILES);
        let sha256 = sha256_hex(&archive);
        let (base, _requests) = serve(200, archive);
        let url = format!("{base}/{PUBLISHED_KEY}.tar.gz");
        let cache = tempfile::tempdir().unwrap();
        let other = Metadata {
            name: "other".to_string(),
            ..source_metadata()
        };
        let failure = failure(fetch_published_into(cache.path(), &url, &sha256, other));
        assert!(failure.contains("does not fit this crate"), "{failure}");
        assert!(!cache.path().join(SEEDED_OUTPUT_DIR).exists());
    }

    #[test]
    fn switching_between_published_and_source_rebuilds_the_entry() {
        // The cache holds the dummy dictionary's source archive, so the
        // build from source needs no network.
        let (cache, archive_sha256) = seeded_cache();
        let (archive, _built) = published_archive(&DICTIONARY_FILES);
        let sha256 = sha256_hex(&archive);
        let (base, requests) = serve(200, archive);
        let url = format!("{base}/{PUBLISHED_KEY}.tar.gz");
        let record = |build_dir: &Path| -> BuildInputs {
            let path = build_inputs_path(&build_dir.join(SEEDED_OUTPUT_DIR));
            serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
        };

        fetch_published_into(cache.path(), &url, &sha256, source_metadata()).unwrap();
        assert!(record(cache.path()).published.is_some());
        build_seeded(cache.path(), archive_sha256).unwrap();
        assert_eq!(record(cache.path()).published, None);
        fetch_published_into(cache.path(), &url, &sha256, source_metadata()).unwrap();
        assert!(record(cache.path()).published.is_some());
        assert_eq!(requests.load(Ordering::SeqCst), 2);
    }

    // The export of a cached dictionary to the directory the crate embeds it
    // from (#1343).

    /// File name of a second dummy archive, whose dictionary has another
    /// entry, so that a build from it replaces the entry of the first one
    /// with other bytes.
    const OTHER_ARCHIVE: &str = "mecab-test-other.tar.gz";

    /// The one entry of the second dummy dictionary.
    const OTHER_LEXICON: &str = "試験,0,0,-500,名詞,一般,*,*,*,*,*,*,*\n";

    /// Writes the second dummy archive into `cache`; returns its SHA-256.
    fn add_other_archive(cache: &Path) -> &'static str {
        write_source_archive(cache, OTHER_ARCHIVE, OTHER_LEXICON)
    }

    /// The parameters that build the second dummy dictionary into the same
    /// entry as [`seeded_params`].
    fn other_params(sha256: &'static str) -> FetchParams {
        FetchParams {
            file_name: OTHER_ARCHIVE,
            ..seeded_params(sha256)
        }
    }

    /// Links a file, as a build exports the entry.
    fn link_file(from: &Path, to: &Path) -> io::Result<()> {
        fs::hard_link(from, to)
    }

    /// The bytes of the files that `embedded_dictionary!` includes, in `dir`.
    fn dictionary_bytes(dir: &Path) -> Vec<Vec<u8>> {
        DICTIONARY_FILES
            .iter()
            .map(|file| {
                fs::read(dir.join(file)).unwrap_or_else(|err| panic!("{file} in {dir:?}: {err}"))
            })
            .collect()
    }

    #[test]
    fn an_export_keeps_its_bytes_when_a_build_from_source_replaces_the_entry() {
        // Another checkout, with other inputs, builds the dictionary from
        // source into the same entry after this build exported it.
        let (cache, sha256) = seeded_cache();
        let other_sha256 = add_other_archive(cache.path());
        let out = tempfile::tempdir().unwrap();
        let export_to = ExportTo {
            dir: out.path(),
            link: &link_file,
        };
        build_with(cache.path(), &seeded_params(sha256), Some(export_to)).unwrap();
        let exported = out.path().join(SEEDED_OUTPUT_DIR);
        let before = dictionary_bytes(&exported);

        build_with(cache.path(), &other_params(other_sha256), None).unwrap();
        assert_eq!(dictionary_bytes(&exported), before);
        assert_ne!(
            dictionary_bytes(&cache.path().join(SEEDED_OUTPUT_DIR)),
            before,
            "the other build did not replace the entry"
        );
    }

    #[test]
    fn an_export_keeps_its_bytes_when_a_published_dictionary_replaces_the_entry() {
        // Another checkout downloads a published dictionary, packed from
        // the second dummy dictionary, into the same entry.
        let (cache, archive_sha256) = seeded_cache();
        let out = tempfile::tempdir().unwrap();
        let export_to = ExportTo {
            dir: out.path(),
            link: &link_file,
        };
        build_with(
            cache.path(),
            &seeded_params(archive_sha256),
            Some(export_to),
        )
        .unwrap();
        let exported = out.path().join(SEEDED_OUTPUT_DIR);
        let before = dictionary_bytes(&exported);

        let other_cache = tempfile::tempdir().unwrap();
        let other_sha256 = add_other_archive(other_cache.path());
        build_with(other_cache.path(), &other_params(other_sha256), None).unwrap();
        let archive = pack_published(
            &other_cache.path().join(SEEDED_OUTPUT_DIR),
            &DICTIONARY_FILES,
        );
        let sha256 = sha256_hex(&archive);
        let (base, _requests) = serve(200, archive);
        let url = format!("{base}/{PUBLISHED_KEY}.tar.gz");
        fetch_published_into(cache.path(), &url, &sha256, source_metadata()).unwrap();
        assert_eq!(dictionary_bytes(&exported), before);
        assert_ne!(
            dictionary_bytes(&cache.path().join(SEEDED_OUTPUT_DIR)),
            before,
            "the download did not replace the entry"
        );
    }

    #[test]
    fn an_export_survives_the_removal_of_the_cache() {
        // The cache is documented as safe to delete; a crate that embeds the
        // export still compiles after it is removed.
        let (cache, sha256) = seeded_cache();
        let out = tempfile::tempdir().unwrap();
        let export_to = ExportTo {
            dir: out.path(),
            link: &link_file,
        };
        build_with(cache.path(), &seeded_params(sha256), Some(export_to)).unwrap();
        let exported = out.path().join(SEEDED_OUTPUT_DIR);
        let before = dictionary_bytes(&exported);

        let cache_path = cache.path().to_path_buf();
        cache.close().unwrap();
        assert!(!cache_path.exists());
        assert_eq!(dictionary_bytes(&exported), before);
        let dictionary = crate::dictionary::Dictionary::load_from_path(&exported).unwrap();
        assert_eq!(dictionary.prefix_dictionary.find_surface("テスト").len(), 1);
    }

    #[test]
    fn an_entry_is_copied_where_it_cannot_be_linked() {
        // As when the cache and the target directory are on different file
        // systems.
        let (cache, sha256) = seeded_cache();
        build_seeded(cache.path(), sha256).unwrap();
        let entry = cache.path().join(SEEDED_OUTPUT_DIR);
        let out = tempfile::tempdir().unwrap();
        let dest = out.path().join(SEEDED_OUTPUT_DIR);
        let no_link =
            |_: &Path, _: &Path| -> io::Result<()> { Err(io::Error::other("no hard links here")) };
        assert_eq!(
            export_entry(&entry, &dest, &no_link).unwrap(),
            Export::Copied("no hard links here".to_string())
        );
        assert_eq!(dictionary_bytes(&dest), dictionary_bytes(&entry));
    }

    #[test]
    fn an_entry_is_linked_where_it_can_be() {
        let (cache, sha256) = seeded_cache();
        build_seeded(cache.path(), sha256).unwrap();
        let entry = cache.path().join(SEEDED_OUTPUT_DIR);
        let out = tempfile::tempdir().unwrap();
        let dest = out.path().join(SEEDED_OUTPUT_DIR);
        assert_eq!(
            export_entry(&entry, &dest, &link_file).unwrap(),
            Export::Linked
        );
        assert_eq!(dictionary_bytes(&dest), dictionary_bytes(&entry));
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            for file in DICTIONARY_FILES {
                assert_eq!(
                    fs::metadata(dest.join(file)).unwrap().ino(),
                    fs::metadata(entry.join(file)).unwrap().ino(),
                    "{file}"
                );
            }
        }
    }

    #[test]
    fn an_entry_replaced_during_the_export_of_a_hit_is_not_embedded() {
        // A hit is exported without the lock. Another build replaces the
        // entry while the third file is linked, so the export holds files of
        // two dictionaries; the check after the export misses, and the build
        // exports again under the lock, after building its own dictionary.
        let (cache, sha256) = seeded_cache();
        let other_sha256 = add_other_archive(cache.path());
        let first = tempfile::tempdir().unwrap();
        let export_to = ExportTo {
            dir: first.path(),
            link: &link_file,
        };
        build_with(cache.path(), &seeded_params(sha256), Some(export_to)).unwrap();
        let own = dictionary_bytes(&first.path().join(SEEDED_OUTPUT_DIR));

        let calls = AtomicUsize::new(0);
        let cache_dir = cache.path().to_path_buf();
        let replacing_link = |from: &Path, to: &Path| -> io::Result<()> {
            if calls.fetch_add(1, Ordering::SeqCst) == 2 {
                build_with(&cache_dir, &other_params(other_sha256), None).unwrap();
            }
            fs::hard_link(from, to)
        };
        let second = tempfile::tempdir().unwrap();
        let export_to = ExportTo {
            dir: second.path(),
            link: &replacing_link,
        };
        build_with(cache.path(), &seeded_params(sha256), Some(export_to)).unwrap();
        assert_eq!(
            dictionary_bytes(&second.path().join(SEEDED_OUTPUT_DIR)),
            own
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2 * DICTIONARY_FILES.len(),
            "the entry was not exported twice"
        );
    }

    #[test]
    fn a_cache_hit_exports_without_waiting_for_the_lock() {
        // As `a_cache_hit_does_not_wait_for_the_lock`, for a build that
        // exports the entry.
        let (cache, sha256) = seeded_cache();
        build_seeded(cache.path(), sha256).unwrap();
        let build_dir = cache.path().to_path_buf();
        let _lock = hold_entry_lock(&build_dir);
        let out = tempfile::tempdir().unwrap();
        let out_dir = out.path().to_path_buf();
        let build = std::thread::spawn(move || {
            let export_to = ExportTo {
                dir: &out_dir,
                link: &link_file,
            };
            build_with(&build_dir, &seeded_params(sha256), Some(export_to))
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !build.is_finished() {
            assert!(
                std::time::Instant::now() < deadline,
                "a cache hit waited for the lock"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        build.join().unwrap().unwrap();
        assert_eq!(
            dictionary_bytes(&out.path().join(SEEDED_OUTPUT_DIR)),
            dictionary_bytes(&cache.path().join(SEEDED_OUTPUT_DIR))
        );
    }
}
