//! Embedding one file that a release publishes, for a crate whose build
//! script cannot make the file itself (#1440).
//!
//! A dictionary crate builds its dictionary from source when it has no pin
//! ([`build_embedded_dictionary`](super::build_embedded_dictionary)). The
//! model of `lindera-dependency` cannot be made that way: its training is in
//! the crate that embeds it, and needs a corpus. So the file comes from one
//! of two places:
//!
//! - the archive a release has put on the page of its version, pinned by the
//!   crate's `prebuilt.json`, which the release writes just before it
//!   publishes the crate, as for a dictionary crate (#1362);
//! - a file on this machine, named by an environment variable of the crate:
//!   what a build from the repository uses, since the repository has no
//!   `prebuilt.json`.
//!
//! A build that has neither stops with an error that says how to get one.

use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use ureq::Agent;

use super::{
    ArchiveFormat, MAX_ROUND, PUBLISHED_BASE_URL_ENV, PrebuiltDictionary, archive_format,
    base_url_from, download_with_retry, is_docs_rs_build, read_prebuilt,
};
use crate::LinderaResult;
use crate::error::LinderaErrorKind;

/// A file that a release publishes in an archive of its own, and a crate
/// embeds.
#[derive(Clone, Copy, Debug)]
pub struct PublishedFile {
    /// The one directory the archive holds, the `key` of the pin, and the
    /// name of the file in messages, such as `lindera-dependency-ipadic`.
    pub key: &'static str,
    /// The file of the crate that pins the archive, such as
    /// `prebuilt-ipadic.json`: a file of its own for each published file,
    /// so that a crate can embed several. It has the format of the
    /// `prebuilt.json` of a dictionary crate.
    pub pin: &'static str,
    /// The file in that directory, such as `dependency.model`.
    pub file: &'static str,
    /// The format version of the file this crate reads. A pin that names
    /// another one is refused before anything is downloaded.
    pub format_version: u32,
    /// The environment variable that names a file on this machine to embed
    /// instead of the published one.
    pub file_env: &'static str,
    /// The name the file gets in `OUT_DIR`, where the crate includes it from.
    pub output: &'static str,
    /// How to make the file, for the error of a build that has neither a pin
    /// nor the variable.
    pub how_to_make: &'static str,
}

/// Where a build takes the file from.
#[derive(Debug, PartialEq, Eq)]
enum FileSource {
    /// A documentation build: an empty file, so that the crate compiles
    /// without a download.
    Dummy,
    /// The file the environment variable names.
    Local(PathBuf),
    /// The archive the crate's `prebuilt.json` pins, and its SHA-256.
    Published {
        /// Where to download the archive.
        url: String,
        /// The SHA-256 the archive must have.
        sha256: String,
    },
}

/// Chooses where the file comes from. The caller reads the environment and
/// the pin, so that every case can be tested without either.
///
/// # Arguments
///
/// * `params` - The file.
/// * `docs_rs` - Whether this is a documentation build.
/// * `local` - The value of [`PublishedFile::file_env`], if set.
/// * `prebuilt` - The crate's pin, if it has one.
///
/// # Returns
///
/// An empty file for a documentation build; else the file of the variable,
/// when it is set and not blank; else the pinned archive. An error when the
/// pin is of another file or of another format version, when it has no
/// SHA-256, and when there is neither a pin nor the variable.
fn choose_file_source(
    params: &PublishedFile,
    docs_rs: bool,
    local: Option<OsString>,
    prebuilt: Option<&PrebuiltDictionary>,
) -> LinderaResult<FileSource> {
    if docs_rs {
        return Ok(FileSource::Dummy);
    }
    if let Some(local) = local.filter(|value| !value.to_string_lossy().trim().is_empty()) {
        return Ok(FileSource::Local(PathBuf::from(local)));
    }
    let key = params.key;
    let file_env = params.file_env;
    let pin = params.pin;
    let Some(prebuilt) = prebuilt else {
        return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
            "{key} is embedded from a release of Lindera, and a build from the repository has \
             none to take it from: {}, and set {file_env} to the file",
            params.how_to_make
        )));
    };
    let hint = format!("set {file_env} to a file to embed instead");
    if prebuilt.key != key {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "{pin} pins {}, and this crate embeds {key}; {hint}",
            prebuilt.key
        )));
    }
    if prebuilt.format_version != params.format_version {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "{pin} pins {key} in the format version {}, and this crate reads the \
             version {}; {hint}",
            prebuilt.format_version,
            params.format_version
        )));
    }
    let Some(sha256) = prebuilt.sha256.clone() else {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "{pin} pins {key} without a SHA-256; {hint}"
        )));
    };
    Ok(FileSource::Published {
        url: prebuilt.url(),
        sha256,
    })
}

/// Downloads the pinned archive, checks its SHA-256, and takes the file out
/// of it.
///
/// # Arguments
///
/// * `params` - The file.
/// * `url` - Where to download the archive; it has to end in `.zip`.
/// * `sha256` - The SHA-256 the archive must have.
///
/// # Returns
///
/// The bytes of `<key>/<file>` in the archive, or an error that names what
/// failed: nothing is embedded from an archive that cannot be checked.
fn fetch_published_file(params: &PublishedFile, url: &str, sha256: &str) -> LinderaResult<Vec<u8>> {
    let key = params.key;
    let hint = format!("set {} to a file to embed instead", params.file_env);
    if !matches!(archive_format(url), Ok(ArchiveFormat::Zip)) {
        return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
            "The published {key} at {url} is not a .zip archive; {hint}"
        )));
    }
    let agent: Agent = Agent::config_builder()
        .user_agent(format!("Lindera/{}", env!("CARGO_PKG_VERSION")))
        .http_status_as_error(false)
        .build()
        .into();
    let content = download_with_retry(&agent, vec![url], MAX_ROUND, sha256).map_err(|err| {
        LinderaErrorKind::Io.with_error(anyhow::anyhow!(
            "Failed to download the published {key} from {url}: {err}; {hint}"
        ))
    })?;
    let name = format!("{key}/{}", params.file);
    let mut bytes = Vec::new();
    zip::ZipArchive::new(Cursor::new(content))
        .and_then(|mut archive| {
            let mut entry = archive.by_name(&name)?;
            entry.read_to_end(&mut bytes)?;
            Ok(())
        })
        .map_err(|err| {
            LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "The published {key} at {url} has no readable {name}: {err}; {hint}"
            ))
        })?;
    Ok(bytes)
}

/// The directives of the build script: what makes cargo run it again.
///
/// # Arguments
///
/// * `params` - The file.
/// * `has_prebuilt` - Whether the crate has a `prebuilt.json`. A directive for
///   a file that does not exist would run the script at every build.
///
/// # Returns
///
/// The lines to print.
fn rerun_directives(params: &PublishedFile, has_prebuilt: bool) -> Vec<String> {
    let mut directives = vec![
        "cargo:rerun-if-changed=build.rs".to_string(),
        format!("cargo:rerun-if-env-changed={}", params.file_env),
        format!("cargo:rerun-if-env-changed={}", super::DOCS_RS_ENV),
    ];
    // Not the variable of the mirror: the file is the same from any host, as
    // for a dictionary.
    if has_prebuilt {
        directives.push(format!("cargo:rerun-if-changed={}", params.pin));
    }
    directives
}

/// The body of the build script of a crate that embeds a published file.
///
/// Writes the file to `OUT_DIR` under [`PublishedFile::output`], from where
/// the crate includes it. It is read, in this order, from:
///
/// 1. nowhere in a documentation build (`DOCS_RS`): the file is empty, and
///    the crate has to answer an empty file with an error, not a panic;
/// 2. the file [`PublishedFile::file_env`] names, when the variable is set;
/// 3. the archive the crate's `prebuilt.json` pins, downloaded and checked
///    against its SHA-256. `LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL`
///    names a mirror of the releases, as for the dictionaries.
///
/// With the embed feature off this registers the script and does nothing
/// else.
///
/// # Arguments
///
/// * `embed_enabled` - Whether the feature that embeds the file is on.
/// * `params` - The file.
///
/// # Returns
///
/// `Ok(())` once the file is in `OUT_DIR`, or when the feature is off; the
/// error otherwise, which stops the build.
pub fn build_embedded_file(
    embed_enabled: bool,
    params: PublishedFile,
) -> Result<(), Box<dyn Error>> {
    if !embed_enabled {
        println!("cargo:rerun-if-changed=build.rs");
        return Ok(());
    }
    let pin = Path::new(params.pin);
    let has_prebuilt = pin.is_file();
    let local = std::env::var_os(params.file_env);
    // Printed before anything can fail, so that a build that failed for want
    // of the variable runs again when it is set.
    for directive in rerun_directives(&params, has_prebuilt) {
        println!("{directive}");
    }
    let base_url = base_url_from(
        PUBLISHED_BASE_URL_ENV,
        std::env::var_os(PUBLISHED_BASE_URL_ENV),
    )?;
    let prebuilt = read_prebuilt(pin)?.map(|prebuilt| prebuilt.with_base_url(base_url.as_deref()));
    let source = choose_file_source(&params, is_docs_rs_build(), local, prebuilt.as_ref())?;
    let bytes = match &source {
        FileSource::Dummy => Vec::new(),
        FileSource::Local(path) => {
            println!("cargo:rerun-if-changed={}", path.display());
            fs::read(path).map_err(|err| {
                LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                    "Failed to read {}, which {} names: {err}",
                    path.display(),
                    params.file_env
                ))
            })?
        }
        FileSource::Published { url, sha256 } => fetch_published_file(&params, url, sha256)?,
    };
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or_else(|| {
        LinderaErrorKind::Io.with_error(anyhow::anyhow!("OUT_DIR environment variable is not set"))
    })?);
    let output = out_dir.join(params.output);
    fs::write(&output, bytes).map_err(|err| {
        LinderaErrorKind::Io.with_error(anyhow::anyhow!(
            "Failed to write {}: {err}",
            output.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::assets::sha256_hex;
    use crate::assets::tests::serve;

    const PARAMS: PublishedFile = PublishedFile {
        key: "lindera-dependency-ipadic",
        pin: "prebuilt-ipadic.json",
        file: "dependency.model",
        format_version: 1,
        file_env: "LINDERA_BUILD_TEST_MODEL_FILE",
        output: "dependency-ipadic.model",
        how_to_make: "build one with the script",
    };

    /// A pin of [`PARAMS`] under `base_url`.
    fn pin(base_url: &str, sha256: Option<&str>) -> PrebuiltDictionary {
        PrebuiltDictionary {
            key: PARAMS.key.to_string(),
            format_version: PARAMS.format_version,
            base_url: base_url.to_string(),
            path: "v7.1.0/lindera-dependency-ipadic-7.1.0.zip".to_string(),
            sha256: sha256.map(str::to_string),
        }
    }

    /// A zip archive with the entries given, as (name, content).
    fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, content) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    /// The message of the error of a choice.
    fn refused(local: Option<&str>, prebuilt: Option<&PrebuiltDictionary>) -> String {
        choose_file_source(&PARAMS, false, local.map(OsString::from), prebuilt)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_documentation_build_embeds_an_empty_file_whatever_else_is_set() {
        let pin = pin("https://example.com", Some("00"));
        assert_eq!(
            choose_file_source(&PARAMS, true, Some("a.model".into()), Some(&pin)).unwrap(),
            FileSource::Dummy
        );
        assert_eq!(
            choose_file_source(&PARAMS, true, None, None).unwrap(),
            FileSource::Dummy
        );
    }

    #[test]
    fn the_file_of_the_variable_comes_before_the_pin() {
        let pin = pin("https://example.com", Some("00"));
        assert_eq!(
            choose_file_source(&PARAMS, false, Some("a.model".into()), Some(&pin)).unwrap(),
            FileSource::Local(PathBuf::from("a.model"))
        );
        assert_eq!(
            choose_file_source(&PARAMS, false, Some("a.model".into()), None).unwrap(),
            FileSource::Local(PathBuf::from("a.model"))
        );
    }

    #[test]
    fn a_blank_variable_is_as_if_it_were_not_set() {
        let pin = pin("https://example.com/", Some("00"));
        assert_eq!(
            choose_file_source(&PARAMS, false, Some("  ".into()), Some(&pin)).unwrap(),
            FileSource::Published {
                url: "https://example.com/v7.1.0/lindera-dependency-ipadic-7.1.0.zip".to_string(),
                sha256: "00".to_string(),
            }
        );
    }

    #[test]
    fn a_build_with_neither_says_how_to_get_the_file() {
        let message = refused(None, None);
        assert!(message.contains("lindera-dependency-ipadic"), "{message}");
        assert!(message.contains("build one with the script"), "{message}");
        assert!(
            message.contains("LINDERA_BUILD_TEST_MODEL_FILE"),
            "{message}"
        );
    }

    #[test]
    fn a_pin_of_another_file_or_format_or_without_a_digest_is_refused() {
        let mut other = pin("https://example.com", Some("00"));
        other.key = "lindera-ipadic".to_string();
        assert!(refused(None, Some(&other)).contains("pins lindera-ipadic"));

        let mut newer = pin("https://example.com", Some("00"));
        newer.format_version = 2;
        let message = refused(None, Some(&newer));
        assert!(message.contains("format version 2"), "{message}");
        assert!(message.contains("version 1"), "{message}");

        let message = refused(None, Some(&pin("https://example.com", None)));
        assert!(message.contains("without a SHA-256"), "{message}");
    }

    #[test]
    fn the_file_is_taken_out_of_the_pinned_archive() {
        let archive = zip_of(&[
            ("lindera-dependency-ipadic/NOTICE.txt", b"notice"),
            ("lindera-dependency-ipadic/dependency.model", b"the model"),
        ]);
        let sha256 = sha256_hex(&archive);
        let (base, requests) = serve(200, archive);
        let url = format!("{base}/v7.1.0/lindera-dependency-ipadic-7.1.0.zip");
        assert_eq!(
            fetch_published_file(&PARAMS, &url, &sha256).unwrap(),
            b"the model"
        );
        assert_eq!(requests.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn an_archive_with_another_digest_is_not_read() {
        let archive = zip_of(&[("lindera-dependency-ipadic/dependency.model", b"the model")]);
        let (base, _) = serve(200, archive);
        let url = format!("{base}/a.zip");
        let message = fetch_published_file(&PARAMS, &url, &"0".repeat(64))
            .unwrap_err()
            .to_string();
        assert!(message.contains("SHA-256 mismatch"), "{message}");
        assert!(
            message.contains("LINDERA_BUILD_TEST_MODEL_FILE"),
            "{message}"
        );
    }

    #[test]
    fn an_archive_without_the_file_is_refused() {
        let archive = zip_of(&[("lindera-dependency-ipadic/NOTICE.txt", b"notice")]);
        let sha256 = sha256_hex(&archive);
        let (base, _) = serve(200, archive);
        let url = format!("{base}/a.zip");
        let message = fetch_published_file(&PARAMS, &url, &sha256)
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("no readable lindera-dependency-ipadic/dependency.model"),
            "{message}"
        );
    }

    #[test]
    fn an_archive_that_is_not_a_zip_is_refused_before_a_download() {
        let (base, requests) = serve(200, b"anything".to_vec());
        let url = format!("{base}/a.tar.gz");
        let message = fetch_published_file(&PARAMS, &url, "00")
            .unwrap_err()
            .to_string();
        assert!(message.contains("not a .zip archive"), "{message}");
        assert_eq!(requests.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn the_script_runs_again_for_what_it_read() {
        assert_eq!(
            rerun_directives(&PARAMS, false),
            [
                "cargo:rerun-if-changed=build.rs",
                "cargo:rerun-if-env-changed=LINDERA_BUILD_TEST_MODEL_FILE",
                "cargo:rerun-if-env-changed=DOCS_RS",
            ]
        );
        assert_eq!(
            rerun_directives(&PARAMS, true).last().map(String::as_str),
            Some("cargo:rerun-if-changed=prebuilt-ipadic.json")
        );
        // The pin is the one the file names: a crate that embeds several
        // files has a pin for each.
        let other = PublishedFile {
            pin: "prebuilt-unidic.json",
            ..PARAMS
        };
        assert_eq!(
            rerun_directives(&other, true).last().map(String::as_str),
            Some("cargo:rerun-if-changed=prebuilt-unidic.json")
        );
    }

    #[test]
    fn an_error_of_a_pin_names_its_file() {
        // A pin of another file, of another format version, and without a
        // SHA-256: each message starts with the file of the pin.
        let mut another = pin("https://example.com", Some("00"));
        another.key = "lindera-dependency-unidic".to_string();
        let mut format = pin("https://example.com", Some("00"));
        format.format_version = PARAMS.format_version + 1;
        let unsigned = pin("https://example.com", None);
        for prebuilt in [another, format, unsigned] {
            let message = choose_file_source(&PARAMS, false, None, Some(&prebuilt))
                .unwrap_err()
                .to_string();
            assert!(message.contains("prebuilt-ipadic.json pins "), "{message}");
        }
    }
}
