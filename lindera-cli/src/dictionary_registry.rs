use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use lindera::LinderaResult;
use lindera::dictionary::{DictionaryKind, Metadata};
use lindera::error::LinderaErrorKind;

/// Dictionary names accepted by `lindera download`.
///
/// This list is independent of the `embed-*` cargo features (unlike
/// `DictionaryKind`, whose variants are feature-gated and absent from a
/// default CLI build) and matches the pre-built dictionary archives published
/// on the GitHub releases page.
pub(crate) const DOWNLOADABLE_DICTIONARIES: [&str; 7] = [
    "ipadic",
    "ipadic-neologd",
    "unidic",
    "sudachidict",
    "ko-dic",
    "cc-cedict",
    "jieba",
];

/// Files that make up a complete pre-built dictionary directory (the same
/// since dictionary format version 2: the system automaton is `dict.trie` +
/// `dict.valsidx`).
pub(crate) const REQUIRED_DICTIONARY_FILES: [&str; 9] = [
    "metadata.json",
    "char_def.bin",
    "matrix.mtx",
    "dict.trie",
    "dict.valsidx",
    "dict.vals",
    "dict.wordsidx",
    "dict.words",
    "unk.bin",
];

/// Environment variable naming the directory that holds the dictionaries,
/// one subdirectory per dictionary: `lindera download <name>` installs to
/// `<dir>/<name>`, and `lindera list` lists the subdirectories (#1303).
///
/// Unrelated to the build-time source cache variable
/// (`LINDERA_BUILD_DICTIONARY_CACHE_DIR`) and to its former name
/// `LINDERA_DICTIONARIES_PATH`, which v7.0.0 removed.
const DICTIONARIES_DIR_ENV: &str = "LINDERA_DICTIONARIES_DIR";

/// The former variable, naming the directory above `dictionaries/`; read as
/// `LINDERA_DICTIONARIES_DIR=<value>/dictionaries`, with a deprecation
/// warning, when [`DICTIONARIES_DIR_ENV`] is not set, until v8.0.0 (#1303).
const DEPRECATED_DATA_DIR_ENV: &str = "LINDERA_DATA_DIR";

/// The subdirectory holding the dictionaries, in the directory named by
/// [`DEPRECATED_DATA_DIR_ENV`] and in the default `lindera` directory.
const DICTIONARIES_SUBDIR: &str = "dictionaries";

/// The dictionaries directory chosen from the environment, and the warning
/// to write about [`DEPRECATED_DATA_DIR_ENV`].
#[derive(Debug, PartialEq, Eq)]
struct DictionariesDirChoice {
    /// The directory, or `None` for the default (see [`dictionaries_dir`]).
    dir: Option<PathBuf>,
    /// The warning to write, if the deprecated variable is set.
    warning: Option<String>,
}

/// Chooses the dictionaries directory from the values of
/// [`DICTIONARIES_DIR_ENV`] and [`DEPRECATED_DATA_DIR_ENV`]. The caller reads
/// the environment, so that every combination can be tested without
/// changing it.
///
/// # Arguments
///
/// * `dictionaries_dir` - The value of `LINDERA_DICTIONARIES_DIR`, if set.
/// * `data_dir` - The value of `LINDERA_DATA_DIR`, if set.
///
/// # Returns
///
/// `LINDERA_DICTIONARIES_DIR` when it is set, with a warning that
/// `LINDERA_DATA_DIR` is ignored if that is set too; otherwise
/// `<LINDERA_DATA_DIR>/dictionaries`, with a deprecation warning that names
/// the value to set instead; and no directory, for the default, when neither
/// is set.
fn choose_dictionaries_dir(
    dictionaries_dir: Option<OsString>,
    data_dir: Option<OsString>,
) -> DictionariesDirChoice {
    match (dictionaries_dir, data_dir) {
        (Some(dir), data_dir) => DictionariesDirChoice {
            dir: Some(PathBuf::from(dir)),
            warning: data_dir.map(|_| {
                format!(
                    "Warning: {DEPRECATED_DATA_DIR_ENV} is ignored because {DICTIONARIES_DIR_ENV} is set; {DEPRECATED_DATA_DIR_ENV} is deprecated and will be removed in v8.0.0"
                )
            }),
        },
        (None, Some(data_dir)) => {
            let dir = PathBuf::from(data_dir).join(DICTIONARIES_SUBDIR);
            let warning = format!(
                "Warning: {DEPRECATED_DATA_DIR_ENV} is deprecated and will be removed in v8.0.0; set {DICTIONARIES_DIR_ENV}={} instead",
                dir.display()
            );
            DictionariesDirChoice {
                dir: Some(dir),
                warning: Some(warning),
            }
        }
        (None, None) => DictionariesDirChoice {
            dir: None,
            warning: None,
        },
    }
}

/// Resolves the directory that holds the dictionaries, writing the warning
/// about [`DEPRECATED_DATA_DIR_ENV`] on stderr. The warning goes through
/// `eprintln!`, as the one about `LINDERA_CONFIG_PATH` does (#1304); call
/// this once per command, and only when the command needs the directory.
///
/// # Returns
///
/// The directory chosen by [`choose_dictionaries_dir`]; by default the OS's
/// local data directory joined with `lindera/dictionaries`
/// (`~/.local/share/lindera/dictionaries` on Linux,
/// `~/Library/Application Support/lindera/dictionaries` on macOS,
/// `%LOCALAPPDATA%\lindera\dictionaries` on Windows). An `Io` error when the
/// local data directory cannot be determined and neither variable is set.
pub(crate) fn dictionaries_dir() -> LinderaResult<PathBuf> {
    let choice = choose_dictionaries_dir(
        env::var_os(DICTIONARIES_DIR_ENV),
        env::var_os(DEPRECATED_DATA_DIR_ENV),
    );
    if let Some(warning) = &choice.warning {
        eprintln!("{warning}");
    }
    if let Some(dir) = choice.dir {
        return Ok(dir);
    }

    dirs::data_local_dir()
        .map(|dir| dir.join("lindera").join(DICTIONARIES_SUBDIR))
        .ok_or_else(|| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "could not determine the local data directory; set {DICTIONARIES_DIR_ENV} to name the dictionaries directory"
            ))
        })
}

/// Checks whether a name refers to a downloadable dictionary.
///
/// # Arguments
///
/// * `name` - Dictionary name to check (e.g. `ipadic`).
///
/// # Returns
///
/// `true` if the name is one of [`DOWNLOADABLE_DICTIONARIES`].
pub(crate) fn is_downloadable_dictionary(name: &str) -> bool {
    DOWNLOADABLE_DICTIONARIES.contains(&name)
}

/// Builds the GitHub release URL of a pre-built dictionary archive.
///
/// The release tag is `v`-prefixed while the asset file name is not
/// (e.g. tag `v5.1.0`, asset `lindera-ipadic-5.1.0.zip`).
///
/// # Arguments
///
/// * `name` - Downloadable dictionary name (e.g. `ipadic`).
/// * `version` - Crate version of the running CLI (e.g. `5.1.0`).
///
/// # Returns
///
/// The absolute download URL of the dictionary archive.
pub(crate) fn download_url(name: &str, version: &str) -> String {
    format!(
        "https://github.com/lindera/lindera/releases/download/v{version}/lindera-{name}-{version}.zip"
    )
}

/// Builds the installation directory of a downloaded dictionary.
///
/// # Arguments
///
/// * `dictionaries_dir` - The directory that holds the dictionaries (see
///   [`dictionaries_dir`]).
/// * `name` - Downloadable dictionary name.
///
/// # Returns
///
/// `<dictionaries_dir>/<name>`, the value to pass to `--dict`.
pub(crate) fn dictionary_dir(dictionaries_dir: &Path, name: &str) -> PathBuf {
    dictionaries_dir.join(name)
}

/// Reads the version of Lindera that built a dictionary directory, from the
/// `lindera_version` field of its `metadata.json` (#1303).
///
/// # Arguments
///
/// * `dir` - The dictionary directory.
///
/// # Returns
///
/// The version, or `None` when `metadata.json` cannot be read or parsed, or
/// lacks the field, as a dictionary built before v7.0.0 does.
pub(crate) fn installed_lindera_version(dir: &Path) -> Option<String> {
    let data = fs::read(dir.join("metadata.json")).ok()?;
    Metadata::load(&data).ok()?.lindera_version
}

/// Lists the required dictionary files missing from a directory.
///
/// # Arguments
///
/// * `dir` - Directory expected to contain a complete pre-built dictionary.
///
/// # Returns
///
/// The names from [`REQUIRED_DICTIONARY_FILES`] that are absent (all of them
/// when `dir` does not exist or is not a directory).
pub(crate) fn missing_dictionary_files(dir: &Path) -> Vec<&'static str> {
    REQUIRED_DICTIONARY_FILES
        .iter()
        .copied()
        .filter(|file| !dir.join(file).is_file())
        .collect()
}

/// Checks whether a directory contains a complete pre-built dictionary.
///
/// # Arguments
///
/// * `dir` - Directory to check.
///
/// # Returns
///
/// `true` when all files from [`REQUIRED_DICTIONARY_FILES`] are present.
pub(crate) fn is_complete_dictionary_dir(dir: &Path) -> bool {
    missing_dictionary_files(dir).is_empty()
}

/// Returns the names of the dictionaries embedded in the binary.
///
/// # Returns
///
/// The names of the `embed-*` features the CLI was built with, such as
/// `ipadic`, in [`DictionaryKind::contained_variants`] order.
pub(crate) fn embedded_dictionary_names() -> Vec<String> {
    DictionaryKind::contained_variants()
        .iter()
        .map(|kind| kind.as_str().to_string())
        .collect()
}

/// Checks the dictionary argument of `lindera tokenize --dict` and
/// `lindera build --system-dict`, which takes `embedded://<kind>`,
/// `file://<path>` or a path, but no longer a dictionary name (#1303).
///
/// 1. A URI containing `://` is returned unchanged (e.g. `embedded://ipadic`).
/// 2. An existing filesystem path is returned unchanged, even when it is
///    spelled like a dictionary name (a directory `./ipadic`).
/// 3. A dictionary name (downloadable or embedded) fails with an error
///    that points to `embedded://<name>` and to the directory where
///    `lindera download <name>` puts the dictionary.
/// 4. Anything else is returned unchanged and handled by the dictionary
///    loader downstream.
///
/// # Arguments
///
/// * `value` - Raw argument value.
/// * `option` - The option the value was given to, for the error message
///   (`--dict` or `--system-dict`).
/// * `embedded` - Names of the dictionaries embedded in the binary (see
///   [`embedded_dictionary_names`]).
/// * `dictionaries_dir` - Resolves the directory that holds the
///   dictionaries (see [`dictionaries_dir`]); called only for a dictionary
///   name.
///
/// # Returns
///
/// The dictionary URI or path to pass to the loader, or an `Args` error for
/// a dictionary name.
pub(crate) fn resolve_dictionary_arg(
    value: &str,
    option: &str,
    embedded: &[String],
    dictionaries_dir: impl FnOnce() -> LinderaResult<PathBuf>,
) -> LinderaResult<String> {
    let is_embedded = embedded.iter().any(|name| name.as_str() == value);
    let is_name = is_downloadable_dictionary(value) || is_embedded;
    if value.contains("://") || Path::new(value).exists() || !is_name {
        return Ok(value.to_string());
    }

    let embedded_note = if is_embedded {
        ""
    } else {
        " (this build does not embed it)"
    };
    let (dir, download_note) = match dictionaries_dir() {
        Ok(dir) => {
            let dir = dictionary_dir(&dir, value);
            let note = if is_complete_dictionary_dir(&dir) || !is_downloadable_dictionary(value) {
                String::new()
            } else {
                format!(" (not downloaded yet; run 'lindera download {value}')")
            };
            (dir.display().to_string(), note)
        }
        Err(_) => (format!("<{DICTIONARIES_DIR_ENV}>/{value}"), String::new()),
    };
    Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
        "{option} takes a URI or a path, and '{value}' names a dictionary: pass embedded://{value} for the dictionary embedded in the CLI{embedded_note}, or the path of a downloaded one, {dir}{download_note}"
    )))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Creates the given files (with dummy content) inside a directory.
    fn create_files(dir: &Path, files: &[&str]) {
        fs::create_dir_all(dir).unwrap();
        for file in files {
            fs::write(dir.join(file), b"test").unwrap();
        }
    }

    #[test]
    fn download_url_builds_expected_release_url() {
        assert_eq!(
            download_url("ipadic", "5.1.0"),
            "https://github.com/lindera/lindera/releases/download/v5.1.0/lindera-ipadic-5.1.0.zip"
        );
    }

    #[test]
    fn downloadable_dictionary_names_are_accepted() {
        for name in DOWNLOADABLE_DICTIONARIES {
            assert!(
                is_downloadable_dictionary(name),
                "{name} should be accepted"
            );
        }
    }

    #[test]
    fn invalid_dictionary_names_are_rejected() {
        for name in ["IPADIC", "", "ipadic2", "embedded://ipadic"] {
            assert!(
                !is_downloadable_dictionary(name),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn the_dictionaries_dir_variable_names_the_directory() {
        let choice = choose_dictionaries_dir(Some(OsString::from("/var/lindera/dict")), None);
        assert_eq!(
            choice,
            DictionariesDirChoice {
                dir: Some(PathBuf::from("/var/lindera/dict")),
                warning: None,
            }
        );
    }

    #[test]
    fn the_deprecated_data_dir_variable_names_its_dictionaries_subdirectory_with_a_warning() {
        let choice = choose_dictionaries_dir(None, Some(OsString::from("/var/lindera")));
        let expected = PathBuf::from("/var/lindera").join("dictionaries");
        assert_eq!(choice.dir.as_ref(), Some(&expected));
        let warning = choice.warning.expect("a deprecation warning");
        assert!(
            warning.contains("LINDERA_DATA_DIR is deprecated"),
            "{warning}"
        );
        assert!(
            warning.contains(&format!("LINDERA_DICTIONARIES_DIR={}", expected.display())),
            "{warning}"
        );
    }

    #[test]
    fn the_dictionaries_dir_variable_wins_over_the_deprecated_one() {
        let choice = choose_dictionaries_dir(
            Some(OsString::from("/var/lindera/dict")),
            Some(OsString::from("/var/lindera")),
        );
        assert_eq!(choice.dir, Some(PathBuf::from("/var/lindera/dict")));
        let warning = choice
            .warning
            .expect("a warning that the old name is ignored");
        assert!(warning.contains("LINDERA_DATA_DIR is ignored"), "{warning}");
    }

    #[test]
    fn no_dictionaries_variable_means_the_default_and_no_warning() {
        assert_eq!(
            choose_dictionaries_dir(None, None),
            DictionariesDirChoice {
                dir: None,
                warning: None,
            }
        );
    }

    #[test]
    fn dictionary_dir_is_the_name_in_the_dictionaries_directory() {
        assert_eq!(
            dictionary_dir(Path::new("/var/lindera/dict"), "ipadic"),
            PathBuf::from("/var/lindera/dict/ipadic")
        );
    }

    #[test]
    fn installed_lindera_version_reads_the_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let metadata = Metadata {
            lindera_version: Some("7.0.0".to_string()),
            ..Metadata::default()
        };
        fs::write(
            tmp.path().join("metadata.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
        assert_eq!(
            installed_lindera_version(tmp.path()).as_deref(),
            Some("7.0.0")
        );
    }

    #[test]
    fn installed_lindera_version_is_none_without_the_field_or_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(installed_lindera_version(tmp.path()), None);

        fs::write(
            tmp.path().join("metadata.json"),
            serde_json::to_vec(&Metadata::default()).unwrap(),
        )
        .unwrap();
        assert_eq!(installed_lindera_version(tmp.path()), None);

        fs::write(tmp.path().join("metadata.json"), b"test").unwrap();
        assert_eq!(installed_lindera_version(tmp.path()), None);
    }

    #[test]
    fn missing_dictionary_files_reports_all_when_dir_is_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = missing_dictionary_files(&tmp.path().join("nonexistent"));
        assert_eq!(missing, REQUIRED_DICTIONARY_FILES.to_vec());
    }

    #[test]
    fn missing_dictionary_files_reports_only_absent_files() {
        let tmp = tempfile::tempdir().unwrap();
        create_files(tmp.path(), &["metadata.json", "char_def.bin"]);
        let missing = missing_dictionary_files(tmp.path());
        assert!(!missing.contains(&"metadata.json"));
        assert!(!missing.contains(&"char_def.bin"));
        assert!(missing.contains(&"matrix.mtx"));
        assert!(missing.contains(&"unk.bin"));
    }

    #[test]
    fn complete_dictionary_dir_is_detected() {
        let tmp = tempfile::tempdir().unwrap();
        create_files(tmp.path(), &REQUIRED_DICTIONARY_FILES);
        assert!(is_complete_dictionary_dir(tmp.path()));
        assert!(missing_dictionary_files(tmp.path()).is_empty());
    }

    /// Stands in for [`dictionaries_dir`] where a value must be passed
    /// through without the dictionaries directory being resolved.
    fn no_dictionaries_dir() -> LinderaResult<PathBuf> {
        panic!("the dictionaries directory should not be resolved")
    }

    #[test]
    fn resolve_passes_uri_through_unchanged() {
        let resolved =
            resolve_dictionary_arg("embedded://ipadic", "--dict", &[], no_dictionaries_dir)
                .unwrap();
        assert_eq!(resolved, "embedded://ipadic");
    }

    #[test]
    fn resolve_passes_existing_filesystem_path_through_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("ipadic");
        fs::create_dir_all(&path).unwrap();
        let arg = path.to_str().unwrap();
        // The path is also given as an embedded name, as `./ipadic` would
        // equal the name `ipadic`: an existing path wins over a name.
        let resolved =
            resolve_dictionary_arg(arg, "--dict", &[arg.to_string()], no_dictionaries_dir).unwrap();
        assert_eq!(resolved, arg);
    }

    #[test]
    fn resolve_passes_unknown_name_through_unchanged() {
        let resolved =
            resolve_dictionary_arg("no-such-dictionary", "--dict", &[], no_dictionaries_dir)
                .unwrap();
        assert_eq!(resolved, "no-such-dictionary");
    }

    /// The message of the error for a dictionary name.
    fn name_error(
        value: &str,
        option: &str,
        embedded: &[String],
        dictionaries_dir: &Path,
    ) -> String {
        let err = resolve_dictionary_arg(value, option, embedded, || {
            Ok(dictionaries_dir.to_path_buf())
        })
        .unwrap_err();
        assert!(
            matches!(err.kind(), LinderaErrorKind::Args),
            "{:?}",
            err.kind()
        );
        err.to_string()
    }

    #[test]
    fn resolve_rejects_a_dictionary_name_with_both_ways_to_name_it() {
        let tmp = tempfile::tempdir().unwrap();
        let message = name_error("ipadic", "--dict", &[], tmp.path());
        assert!(message.contains("--dict"), "{message}");
        assert!(message.contains("embedded://ipadic"), "{message}");
        assert!(
            message.contains("this build does not embed it"),
            "{message}"
        );
        let dir = dictionary_dir(tmp.path(), "ipadic");
        assert!(message.contains(&dir.display().to_string()), "{message}");
        assert!(message.contains("lindera download ipadic"), "{message}");
    }

    #[test]
    fn resolve_rejects_an_embedded_and_downloaded_name_without_the_notes() {
        let tmp = tempfile::tempdir().unwrap();
        create_files(
            &dictionary_dir(tmp.path(), "ipadic"),
            &REQUIRED_DICTIONARY_FILES,
        );
        let message = name_error(
            "ipadic",
            "--system-dict",
            &["ipadic".to_string()],
            tmp.path(),
        );
        assert!(message.contains("--system-dict"), "{message}");
        assert!(message.contains("embedded://ipadic"), "{message}");
        assert!(!message.contains("does not embed"), "{message}");
        assert!(!message.contains("lindera download"), "{message}");
    }

    #[test]
    fn resolve_rejects_an_embedded_name_that_is_not_downloadable() {
        let tmp = tempfile::tempdir().unwrap();
        let message = name_error(
            "custom-dic",
            "--dict",
            &["custom-dic".to_string()],
            tmp.path(),
        );
        assert!(message.contains("embedded://custom-dic"), "{message}");
        assert!(!message.contains("lindera download"), "{message}");
    }

    #[test]
    fn resolve_names_the_variable_when_the_dictionaries_directory_is_unknown() {
        let err = resolve_dictionary_arg("ipadic", "--dict", &[], || {
            Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!("no local data directory")))
        })
        .unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("<LINDERA_DICTIONARIES_DIR>/ipadic"),
            "{message}"
        );
    }
}
