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
use crate::builder::context_id_remap::{CTX_FREQ_FILE_ENV, resolve_context_id_freq};
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

    /// MD5 hash of the file
    pub md5_hash: &'static str,
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
/// limit does not apply here. The MD5 check below is what guards against a
/// truncated or substituted download.
const BODY_LIMIT: u64 = u64::MAX;

fn download_with_retry(
    agent: &Agent,
    download_urls: Vec<&str>,
    max_rounds: usize,
    expected_md5: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if download_urls.is_empty() {
        return Err("No download URLs provided".into());
    }

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
                        Ok(content) => {
                            // Calculate MD5 hash
                            let mut context = Context::new();
                            context.consume(&content);
                            let actual_md5 = format!("{:x}", context.finalize());

                            debug!("Expected MD5: {expected_md5}");
                            debug!("Actual   MD5: {actual_md5}");

                            if actual_md5 == expected_md5 {
                                debug!("MD5 check passed from {url}");
                                return Ok(content);
                            } else {
                                warn!(
                                    "MD5 mismatch from {url}, Expected {expected_md5}, got {actual_md5}"
                                );
                                // continue to next url
                            }
                        }
                        Err(e) => {
                            warn!("Failed to download content from {url}: {e}");
                            // continue to next url
                        }
                    }
                }
                Ok(resp) => {
                    warn!("HTTP download failed from {}: HTTP {}", url, resp.status());
                    // continue to next url
                }
                Err(e) => {
                    warn!("Request error from {url}: {e}");
                    // continue to next url
                }
            }
        }

        sleep(Duration::from_secs(1));
    }

    error!("All {max_rounds} attempts failed");
    Err("Failed to download a valid file from all sources".into())
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
    /// MD5 of the source archive ([`FetchParams::md5_hash`]).
    archive_md5: String,
    /// Directory the archive extracts to ([`FetchParams::input_dir`]).
    input_dir: String,
    /// Sub-directory of the extracted archive the build reads
    /// ([`FetchParams::src_subdir`]).
    src_subdir: Option<String>,
    /// MD5 of the context-ID frequency histogram the build ranks context IDs
    /// with, or `None` when it reads none (`connection_id_mapping` off, or no
    /// histogram found).
    context_id_freq_md5: Option<String>,
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

/// The inputs of the build `fetch` is about to run, to compare with a cached
/// dictionary's record.
///
/// # Arguments
///
/// * `params` - The source archive and directory layout of the build.
/// * `builder` - The builder, for its metadata and bundled histogram.
/// * `builder_digest` - This crate's source digest ([`BUILDER_SOURCE_DIGEST`]).
/// * `ctx_freq_env` - The value of [`CTX_FREQ_FILE_ENV`], if set.
///
/// # Returns
///
/// The inputs, `None` when `builder_digest` is `None`, or an error when the
/// histogram the build would read cannot be read.
fn expected_build_inputs(
    params: &FetchParams,
    builder: &DictionaryBuilder,
    builder_digest: Option<&str>,
    ctx_freq_env: Option<OsString>,
) -> LinderaResult<Option<BuildInputs>> {
    let Some(builder_digest) = builder_digest else {
        return Ok(None);
    };
    let context_id_freq_md5 = if builder.metadata().connection_id_mapping {
        resolve_context_id_freq(ctx_freq_env, builder.context_id_freq())
            .map(|path| {
                file_md5(&path).map_err(|err| {
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
        archive_md5: params.md5_hash.to_string(),
        input_dir: params.input_dir.to_string(),
        src_subdir: params.src_subdir.map(str::to_string),
        context_id_freq_md5,
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
/// output directory with two changes: it stamps `format_version` (which the
/// caller checks separately) and, when `connection_id_mapping` is on, embeds
/// the computed `context_id_map`. Everything else is copied verbatim, so any
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
    const BUILD_DERIVED_FIELDS: [&str; 2] = ["format_version", "context_id_map"];

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
/// The file named by `LINDERA_CTX_FREQ_FILE` is watched too, under the same
/// rule: only when it exists. Without that, a new histogram written to the
/// same path would not rerun the build script, and the dictionary would keep
/// the old ranking.
///
/// # Arguments
///
/// * `ships_context_id_freq` - Whether the crate has a `context_id_freq.txt`.
/// * `ctx_freq_override` - The value of `LINDERA_CTX_FREQ_FILE`, if set.
///
/// # Returns
///
/// The directives, one per line, in the order the build script prints them.
fn rerun_directives(ships_context_id_freq: bool, ctx_freq_override: Option<&Path>) -> Vec<String> {
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
    if let Some(path) = ctx_freq_override.filter(|path| path.is_file()) {
        directives.push(format!("cargo:rerun-if-changed={}", path.display()));
    }
    directives.extend([
        format!("cargo:rerun-if-env-changed={CTX_FREQ_FILE_ENV}"),
        format!("cargo:rerun-if-env-changed={CACHE_DIR_ENV}"),
        format!("cargo:rerun-if-env-changed={DOCS_RS_ENV}"),
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
/// Builds that share the cache write each dictionary one at a time: a build
/// that finds another build of the same dictionary writing it waits, and then
/// uses the dictionary that build left in place (see `fetch_into`).
///
/// # Arguments
///
/// * `params` - Describes the asset to fetch (archive name, mirrors, MD5 hash)
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
    let ctx_freq_env = std::env::var_os(CTX_FREQ_FILE_ENV);
    for directive in rerun_directives(
        Path::new(CONTEXT_ID_FREQ_FILE).is_file(),
        ctx_freq_env.as_deref().map(Path::new),
    ) {
        println!("{directive}");
    }

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
        (
            PathBuf::from(std::env::var_os("OUT_DIR").ok_or_else(|| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!("OUT_DIR environment variable is not set"))
            })?), /* ex) target/debug/build/<pkg>/out */
            false,
        )
    };

    // environment variable passed to dependents, that will actually be used to include the dictionary in the library
    println!("cargo::rustc-env=LINDERA_WORKDIR={}", build_dir.display());

    fetch_into(
        &build_dir,
        is_cache,
        is_docs_rs_build(),
        &params,
        &builder,
        purpose,
        ctx_freq_env,
    )
}

/// [`fetch_for`] once the build directory is known: fetches the source and
/// builds the dictionary into `build_dir`, unless the cache there already
/// holds it.
///
/// In a cache, one build at a time writes a dictionary's entry, from the
/// cache check to the record of its inputs, under [`lock_cache_entry`]. The
/// entry is checked first without the lock, so a cache hit neither waits for
/// a build of the same dictionary nor needs a writable cache; on a miss, it
/// is checked again once the lock is held, so a build that waited uses the
/// dictionary the other build left instead of building it again.
///
/// The lock covers the writers only: the compile that embeds the dictionary
/// reads it after the build script has exited. That is safe because a build
/// with the same inputs never rewrites an entry that is current. A build
/// without this crate's source digest rebuilds every time and so does rewrite
/// it, as the `cargo:warning` it prints says; builds with different inputs
/// rebuild each other's entries, which is why the cache directory should not
/// be shared between checkouts.
///
/// # Arguments
///
/// * `build_dir` - The directory to build in: a cache directory, or `OUT_DIR`.
/// * `is_cache` - Whether `build_dir` is a cache, whose entries are reused.
/// * `docs_rs` - Whether to build a dummy dictionary from
///   `params.dummy_input` instead of the downloaded one.
/// * `params` - Describes the asset to fetch and the directory layout.
/// * `builder` - Dictionary builder.
/// * `purpose` - Whether the dictionary is embedded or only fills the cache.
/// * `ctx_freq_env` - The value of [`CTX_FREQ_FILE_ENV`], if set.
///
/// # Returns
///
/// As for [`fetch`].
fn fetch_into(
    build_dir: &Path,
    is_cache: bool,
    docs_rs: bool,
    params: &FetchParams,
    builder: &DictionaryBuilder,
    purpose: CachePurpose,
    ctx_freq_env: Option<OsString>,
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
        expected_build_inputs(params, builder, BUILDER_SOURCE_DIGEST, ctx_freq_env)?
    } else {
        None
    };
    // Bound to a name so that the lock is held until this function returns,
    // after the input directory is removed.
    let _entry_lock = if is_cache {
        if expected.is_none() && purpose == CachePurpose::Embed {
            println!(
                "cargo:warning=the lindera-dictionary source digest is unavailable, \
                 so {} is rebuilt rather than reused",
                output_dir.display()
            );
        }
        let decide = || cache_decision(&output_dir, purpose, expected.as_ref(), builder.metadata());
        let report_hit = |hit: CacheHit| {
            println!(
                "{CACHE_REPORT_PREFIX} using {}: {hit}",
                output_dir.display()
            );
        };
        if let Ok(hit) = decide() {
            report_hit(hit);
            return Ok(());
        }
        let lock = lock_cache_entry(build_dir, params.output_dir)?;
        // Another build may have built the dictionary while this one waited.
        match decide() {
            Ok(hit) => {
                report_hit(hit);
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

    if docs_rs {
        create_dummy_dictionary_source(&input_dir, params.src_subdir, params.dummy_input)?;
    } else {
        // Source file path for build package
        let source_path_for_build = &build_dir.join(params.file_name);

        // Check if source file already exists and is valid
        let need_download = if source_path_for_build.exists() {
            debug!(
                "Found existing source file: {}",
                source_path_for_build.display()
            );

            // Verify MD5 hash
            let actual_md5 = file_md5(source_path_for_build).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to read source file for MD5 check: {source_path_for_build:?}"
                    ))
            })?;

            if actual_md5 == params.md5_hash {
                debug!("MD5 check passed for cached file. Skipping download.");
                false
            } else {
                warn!(
                    "MD5 mismatch for cached file. Expected: {}, Actual: {}",
                    params.md5_hash, actual_md5
                );
                // Remove invalid file
                fs::remove_file(source_path_for_build).map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context(format!(
                            "Failed to remove invalid source file: {source_path_for_build:?}"
                        ))
                })?;
                true
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
                params.md5_hash,
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
        empty_directory(&input_dir)?;

        rename_directory(&tmp_extracted_path, &input_dir)?;

        let _ = fs::remove_dir_all(&tmp_extract_path);
    }

    let tmp_output_path = build_dir.join(format!("tmp-output-{}", params.output_dir));
    let _ = fs::remove_dir_all(&tmp_output_path);

    let build_input_dir = match params.src_subdir {
        Some(subdir) => input_dir.join(subdir),
        None => input_dir.clone(),
    };

    builder
        .build_dictionary(&build_input_dir, &tmp_output_path)
        .map_err(|err| {
            LinderaErrorKind::Build
                .with_error(anyhow::anyhow!("{err}"))
                .add_context("Failed to build dictionary")
        })?;

    // Empty the output directory
    empty_directory(&output_dir)?;

    // Rename tmp_output_path to output_dir
    rename_directory(&tmp_output_path, &output_dir)?;

    // Record the inputs last, once the dictionary is in place, so that an
    // interrupted build leaves no record and the next build rebuilds.
    if let Some(inputs) = &expected {
        write_build_inputs(&output_dir, inputs)?;
    }

    let _ = fs::remove_dir_all(input_dir);

    Ok(())
}

/// Name of the optional per-dictionary context-ID access-frequency histogram,
/// shipped in the dictionary crate root next to `metadata.json`. Produced by the
/// `ctxfreq` instrumentation (see the `ctxfreq_dump` example) and consumed when
/// `connection_id_mapping` is enabled.
const CONTEXT_ID_FREQ_FILE: &str = "context_id_freq.txt";

/// Shared body of every per-dictionary crate's `build.rs`.
///
/// Reads `metadata.json` from the crate root, fetches and builds the
/// dictionary described by `params`, and embeds the result under
/// `LINDERA_WORKDIR`.
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
/// * `params` - Describes the asset to fetch (archive name, mirrors, MD5 hash)
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
    const ALWAYS: [&str; 6] = [
        "cargo:rerun-if-changed=build.rs",
        "cargo:rerun-if-changed=Cargo.toml",
        "cargo:rerun-if-changed=metadata.json",
        "cargo:rerun-if-env-changed=LINDERA_CTX_FREQ_FILE",
        "cargo:rerun-if-env-changed=LINDERA_BUILD_DICTIONARY_CACHE_DIR",
        "cargo:rerun-if-env-changed=DOCS_RS",
    ];

    #[test]
    fn rerun_directives_skip_a_missing_context_id_freq_file() {
        // A missing `rerun-if-changed` path would rerun the build script,
        // and rebuild the dictionary, on every build (#1069).
        assert_eq!(super::rerun_directives(false, None), ALWAYS);
    }

    #[test]
    fn rerun_directives_watch_a_shipped_context_id_freq_file() {
        let mut expected = ALWAYS.to_vec();
        expected.insert(3, "cargo:rerun-if-changed=context_id_freq.txt");
        assert_eq!(super::rerun_directives(true, None), expected);
    }

    #[test]
    fn rerun_directives_watch_an_existing_ctx_freq_override() {
        // A new histogram written to the same path must rerun the build.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("freq.txt");
        std::fs::write(&path, "").unwrap();
        let mut expected = ALWAYS.map(String::from).to_vec();
        expected.insert(3, format!("cargo:rerun-if-changed={}", path.display()));
        assert_eq!(super::rerun_directives(false, Some(&path)), expected);
    }

    #[test]
    fn rerun_directives_skip_a_missing_ctx_freq_override() {
        // Same rule as the shipped file (#1069): a missing path would rerun
        // the build script on every build.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.txt");
        assert_eq!(super::rerun_directives(false, Some(&path)), ALWAYS);
    }

    use super::*;
    use crate::dictionary::context_id_map::ContextIdMap;

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
        // The build stamps `format_version` and, with connection-ID mapping,
        // embeds the computed `context_id_map`; neither is in the crate's
        // metadata.json, so neither may count as a difference.
        let cached = Metadata {
            format_version: DICTIONARY_FORMAT_VERSION,
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
            archive_md5: "fedcba9876543210fedcba9876543210".to_string(),
            input_dir: "mecab-ipadic-2.7.0-20250920".to_string(),
            src_subdir: None,
            context_id_freq_md5: None,
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
            ("archive_md5", |i| i.archive_md5.replace_range(0..1, "0")),
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
            md5_hash: "fedcba9876543210fedcba9876543210",
        }
    }

    #[test]
    fn expected_inputs_follow_the_histogram_contents_with_mapping() {
        let dir = tempfile::tempdir().unwrap();
        let (builder, histogram) = builder_with_histogram(dir.path(), true);
        let before = expected_build_inputs(&fetch_params(), &builder, Some("d"), None)
            .unwrap()
            .unwrap();
        fs::write(&histogram, "1 1 6\n").unwrap();
        let after = expected_build_inputs(&fetch_params(), &builder, Some("d"), None)
            .unwrap()
            .unwrap();
        assert!(before.context_id_freq_md5.is_some());
        assert_ne!(before.context_id_freq_md5, after.context_id_freq_md5);
    }

    #[test]
    fn expected_inputs_follow_the_override_histogram() {
        // The build reads `LINDERA_CTX_FREQ_FILE` before the bundled file, so
        // the record must hash that one.
        let dir = tempfile::tempdir().unwrap();
        let (builder, _) = builder_with_histogram(dir.path(), true);
        let override_path = dir.path().join("override.txt");
        fs::write(&override_path, "1 1 7\n").unwrap();
        let bundled = expected_build_inputs(&fetch_params(), &builder, Some("d"), None)
            .unwrap()
            .unwrap();
        let overridden = expected_build_inputs(
            &fetch_params(),
            &builder,
            Some("d"),
            Some(override_path.clone().into_os_string()),
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
        let inputs = expected_build_inputs(&fetch_params(), &builder, Some("d"), None)
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
        let inputs = expected_build_inputs(&params, &builder, Some("digest"), None)
            .unwrap()
            .unwrap();
        assert_eq!(
            inputs,
            BuildInputs {
                builder_version: env!("CARGO_PKG_VERSION").to_string(),
                builder_digest: "digest".to_string(),
                archive_file_name: params.file_name.to_string(),
                archive_md5: params.md5_hash.to_string(),
                input_dir: params.input_dir.to_string(),
                src_subdir: Some("dict-src".to_string()),
                context_id_freq_md5: None,
            }
        );
    }

    #[test]
    fn no_expected_inputs_without_a_source_digest() {
        let dir = tempfile::tempdir().unwrap();
        let (builder, _) = builder_with_histogram(dir.path(), true);
        assert_eq!(
            expected_build_inputs(&fetch_params(), &builder, None, None).unwrap(),
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
    /// archive's MD5, for [`seeded_params`].
    fn seeded_cache() -> (tempfile::TempDir, &'static str) {
        let cache = tempfile::tempdir().unwrap();
        let sources = tempfile::tempdir().unwrap();
        let source_dir = sources.path().join(SEEDED_INPUT_DIR);
        create_dummy_dictionary_source(&source_dir, None, SEEDED_LEXICON).unwrap();
        let archive_path = cache.path().join(SEEDED_ARCHIVE);
        let encoder = flate2::write::GzEncoder::new(
            File::create(&archive_path).unwrap(),
            flate2::Compression::fast(),
        );
        let mut archive = tar::Builder::new(encoder);
        archive
            .append_dir_all(SEEDED_INPUT_DIR, &source_dir)
            .unwrap();
        archive.into_inner().unwrap().finish().unwrap();
        let md5 = file_md5(&archive_path).unwrap();
        (cache, Box::leak(md5.into_boxed_str()))
    }

    /// The parameters that build the dummy dictionary of [`seeded_cache`].
    fn seeded_params(md5: &'static str) -> FetchParams {
        FetchParams {
            file_name: SEEDED_ARCHIVE,
            input_dir: SEEDED_INPUT_DIR,
            src_subdir: None,
            output_dir: SEEDED_OUTPUT_DIR,
            dummy_input: "",
            download_urls: &[],
            md5_hash: md5,
        }
    }

    /// Builds the dummy dictionary into the cache `build_dir`, to embed it.
    fn build_seeded(build_dir: &Path, md5: &'static str) -> LinderaResult<()> {
        fetch_into(
            build_dir,
            true,
            false,
            &seeded_params(md5),
            &DictionaryBuilder::new(source_metadata()),
            CachePurpose::Embed,
            None,
        )
    }

    /// The record a build of the dummy dictionary writes.
    fn seeded_inputs(md5: &'static str) -> BuildInputs {
        expected_build_inputs(
            &seeded_params(md5),
            &DictionaryBuilder::new(source_metadata()),
            BUILDER_SOURCE_DIGEST,
            None,
        )
        .unwrap()
        .unwrap()
    }

    /// Writes a current entry for the dummy dictionary, standing in for the
    /// one another build leaves, with a marker file that a rebuild removes;
    /// returns the marker's path.
    fn write_current_entry(build_dir: &Path, md5: &'static str) -> PathBuf {
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
        write_build_inputs(&output_dir, &seeded_inputs(md5)).unwrap();
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
        let (cache, md5) = seeded_cache();
        let build_dir = cache.path().to_path_buf();
        let lock = hold_entry_lock(&build_dir);
        let waiting = {
            let build_dir = build_dir.clone();
            std::thread::spawn(move || build_seeded(&build_dir, md5))
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
        let marker = write_current_entry(&build_dir, md5);
        drop(lock);
        waiting.join().unwrap().unwrap();
        assert!(marker.exists(), "the entry was built again");
    }

    #[test]
    fn a_cache_hit_does_not_wait_for_the_lock() {
        // A current entry is used without the lock, so that a read-only
        // cache works and builds that only read it do not queue up.
        let (cache, md5) = seeded_cache();
        let build_dir = cache.path().to_path_buf();
        let marker = write_current_entry(&build_dir, md5);
        let _lock = hold_entry_lock(&build_dir);
        let build = {
            let build_dir = build_dir.clone();
            std::thread::spawn(move || build_seeded(&build_dir, md5))
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
            let (cache, md5) = seeded_cache();
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(BUILDS));
            let builds: Vec<_> = (0..BUILDS)
                .map(|_| {
                    let build_dir = cache.path().to_path_buf();
                    let barrier = barrier.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        build_seeded(&build_dir, md5)
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
                decide(&output_dir, Some(&seeded_inputs(md5))),
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
}
