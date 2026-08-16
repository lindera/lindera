use std::path::{Path, PathBuf};

use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;

/// Dictionary names accepted by `lindera download`.
///
/// This list is independent of the `embed-*` cargo features (unlike
/// `DictionaryKind`, whose variants are feature-gated and absent from a
/// default CLI build) and matches the pre-built dictionary archives published
/// on the GitHub releases page.
pub(crate) const DOWNLOADABLE_DICTIONARIES: [&str; 6] = [
    "ipadic",
    "ipadic-neologd",
    "unidic",
    "ko-dic",
    "cc-cedict",
    "jieba",
];

/// Files that make up a complete pre-built dictionary directory
/// (dictionary format version 2: the system automaton is `dict.trie` +
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

/// Environment variable overriding the base application data directory used
/// for downloaded dictionaries.
///
/// Unrelated to the build-time source cache variable
/// (`LINDERA_BUILD_DICTIONARY_CACHE_DIR`).
pub(crate) const DATA_DIR_ENV: &str = "LINDERA_DATA_DIR";

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

/// Resolves the base application data directory for Lindera.
///
/// # Arguments
///
/// * `env_override` - Value of the `LINDERA_DATA_DIR` environment variable,
///   if set. The caller reads the environment so that this function stays
///   testable without touching process state.
///
/// # Returns
///
/// The override as-is when present; otherwise the OS-standard local
/// application data directory joined with `lindera` (e.g.
/// `~/.local/share/lindera` on Linux, `~/Library/Application Support/lindera`
/// on macOS, `%LOCALAPPDATA%\lindera` on Windows). An `Io` error when neither
/// is available.
pub(crate) fn base_data_dir(env_override: Option<PathBuf>) -> LinderaResult<PathBuf> {
    if let Some(dir) = env_override {
        return Ok(dir);
    }

    dirs::data_local_dir()
        .map(|dir| dir.join("lindera"))
        .ok_or_else(|| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "could not determine the application data directory; set {DATA_DIR_ENV} to override it"
            ))
        })
}

/// Builds the installation directory of a downloaded dictionary.
///
/// # Arguments
///
/// * `base` - Base data directory (see [`base_data_dir`]).
/// * `version` - Crate version of the running CLI.
/// * `name` - Downloadable dictionary name.
///
/// # Returns
///
/// `<base>/dictionaries/<version>/lindera-<name>`. The `lindera-<name>`
/// component matches the top-level directory inside the release archive.
pub(crate) fn dictionary_dir(base: &Path, version: &str, name: &str) -> PathBuf {
    base.join("dictionaries")
        .join(version)
        .join(format!("lindera-{name}"))
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

/// Resolves the `--dict` argument of `lindera tokenize`.
///
/// Resolution order, preserving backward compatibility:
///
/// 1. A URI containing `://` is returned unchanged (e.g. `embedded://ipadic`).
/// 2. An existing filesystem path is returned unchanged (paths always win
///    over dictionary names).
/// 3. A downloadable dictionary name resolves to its downloaded directory; a
///    missing or incomplete directory produces an error suggesting
///    `lindera download`.
/// 4. Anything else is returned unchanged and handled by the dictionary
///    loader downstream.
///
/// # Arguments
///
/// * `dict` - Raw `--dict` argument value.
/// * `env_override` - Value of the `LINDERA_DATA_DIR` environment variable,
///   if set.
/// * `version` - Crate version of the running CLI.
///
/// # Returns
///
/// The dictionary URI or path to pass to the tokenizer builder.
pub(crate) fn resolve_dictionary_arg(
    dict: &str,
    env_override: Option<PathBuf>,
    version: &str,
) -> LinderaResult<String> {
    if dict.contains("://") || Path::new(dict).exists() || !is_downloadable_dictionary(dict) {
        return Ok(dict.to_string());
    }

    let dir = dictionary_dir(&base_data_dir(env_override)?, version, dict);
    let missing = missing_dictionary_files(&dir);
    if missing.is_empty() {
        return dir.into_os_string().into_string().map_err(|path| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "dictionary path is not valid UTF-8: {}",
                PathBuf::from(path).display()
            ))
        });
    }

    if dir.is_dir() {
        Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "dictionary '{dict}' at {} is incomplete (missing: {}). Run 'lindera download {dict} --force' to re-download it.",
            dir.display(),
            missing.join(", ")
        )))
    } else {
        Err(LinderaErrorKind::NotFound.with_error(anyhow::anyhow!(
            "dictionary '{dict}' is not downloaded (looked in {}). Run 'lindera download {dict}' to fetch it.",
            dir.display()
        )))
    }
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
    fn base_data_dir_prefers_env_override() {
        let dir = base_data_dir(Some(PathBuf::from("/custom/data"))).unwrap();
        assert_eq!(dir, PathBuf::from("/custom/data"));
    }

    #[test]
    fn base_data_dir_defaults_to_local_data_dir() {
        if let Some(expected) = dirs::data_local_dir() {
            assert_eq!(base_data_dir(None).unwrap(), expected.join("lindera"));
        }
    }

    #[test]
    fn dictionary_dir_uses_versioned_layout() {
        assert_eq!(
            dictionary_dir(Path::new("/base"), "5.1.0", "ipadic"),
            PathBuf::from("/base/dictionaries/5.1.0/lindera-ipadic")
        );
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

    #[test]
    fn resolve_passes_uri_through_unchanged() {
        let resolved = resolve_dictionary_arg("embedded://ipadic", None, "5.1.0").unwrap();
        assert_eq!(resolved, "embedded://ipadic");
    }

    #[test]
    fn resolve_prefers_existing_filesystem_path() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("ipadic");
        fs::create_dir_all(&path).unwrap();
        let arg = path.to_str().unwrap();
        let resolved = resolve_dictionary_arg(arg, None, "5.1.0").unwrap();
        assert_eq!(resolved, arg);
    }

    #[test]
    fn resolve_returns_downloaded_dictionary_path() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = dictionary_dir(tmp.path(), "5.1.0", "ipadic");
        create_files(&dir, &REQUIRED_DICTIONARY_FILES);
        let resolved =
            resolve_dictionary_arg("ipadic", Some(tmp.path().to_path_buf()), "5.1.0").unwrap();
        assert_eq!(resolved, dir.to_str().unwrap());
    }

    #[test]
    fn resolve_suggests_download_when_not_downloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let err =
            resolve_dictionary_arg("ipadic", Some(tmp.path().to_path_buf()), "5.1.0").unwrap_err();
        let message = format!("{err}");
        assert!(message.contains("lindera download ipadic"), "{message}");
    }

    #[test]
    fn resolve_suggests_force_when_incomplete() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = dictionary_dir(tmp.path(), "5.1.0", "ipadic");
        create_files(&dir, &["metadata.json"]);
        let err =
            resolve_dictionary_arg("ipadic", Some(tmp.path().to_path_buf()), "5.1.0").unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("lindera download ipadic --force"),
            "{message}"
        );
        assert!(message.contains("unk.bin"), "{message}");
    }

    #[test]
    fn resolve_passes_unknown_name_through_unchanged() {
        let resolved = resolve_dictionary_arg("no-such-dictionary", None, "5.1.0").unwrap();
        assert_eq!(resolved, "no-such-dictionary");
    }
}
