use std::fs;
use std::path::{Path, PathBuf};

use lindera::LinderaResult;
use lindera::dictionary::load_dictionary;
use lindera_cli::get_version;
use lindera_crf::console::{Stream, write_line};

use super::{io_err, write_stdout};
use crate::dictionary_registry::{
    Downloadable, MODEL_METADATA_FILE, dictionaries_dir, embedded_dictionary_names,
    installed_lindera_version, is_complete_dictionary_dir,
};

/// Column headers of the dictionary listing table.
const HEADERS: [&str; 4] = ["NAME", "VERSION", "STATUS", "DICT"];

/// Column headers of the table of the models of the dependency parser. The
/// last column is the value of `model` of the token filter
/// `japanese_dependency`.
const MODEL_HEADERS: [&str; 4] = ["NAME", "VERSION", "STATUS", "MODEL"];

/// Shown in the `VERSION` column when the dictionary does not say which
/// version of Lindera built it, as a dictionary built before v7.0.0 does.
const UNKNOWN_VERSION: &str = "unknown";

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "List the embedded dictionaries and those in LINDERA_DICTIONARIES_DIR, and the models of the dependency parser",
    version = get_version(),
)]
/// Arguments for the `list` subcommand.
pub struct ListArgs {}

/// State of a listed dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// Complete, and built by the CLI's version.
    Ok,
    /// Required files are missing.
    Incomplete,
    /// Complete, but built by another version of Lindera, or by one that
    /// does not say.
    OtherVersion,
}

impl Status {
    /// Returns the text of the `STATUS` column.
    ///
    /// # Returns
    ///
    /// `ok`, `incomplete` or `other version`.
    fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Incomplete => "incomplete",
            Status::OtherVersion => "other version",
        }
    }
}

/// One row of the dictionary listing.
#[derive(Debug, PartialEq, Eq)]
struct DictionaryRow {
    /// Dictionary name: the embedded kind (e.g. `ipadic`), or the name of the
    /// directory in the dictionaries directory.
    name: String,
    /// Version of Lindera that built the dictionary, if it says.
    version: Option<String>,
    /// State of the dictionary.
    status: Status,
    /// The value to pass to `--dict`: `embedded://<name>` or the directory.
    dict: String,
}

/// The dictionaries found in the dictionaries directory.
#[derive(Debug, Default, PartialEq, Eq)]
struct DirectoryListing {
    /// One row per dictionary, by name.
    rows: Vec<DictionaryRow>,
    /// One row per model of the dependency parser, by name: the
    /// subdirectories with a `model.json` (#1440). The last cell of a row
    /// is the model file.
    models: Vec<DictionaryRow>,
    /// Subdirectories with neither `metadata.json` nor `model.json`, by
    /// name.
    skipped: Vec<PathBuf>,
}

/// Runs the `list` subcommand.
///
/// Prints a table of the dictionaries embedded in the binary (via `embed-*`
/// cargo features) and of those in `LINDERA_DICTIONARIES_DIR` (see
/// `lindera download`), each with the version of Lindera that built it, its
/// state, and the value to pass to `--dict` (#1303). Subdirectories without
/// `metadata.json` are reported on stderr instead, as is a dictionaries
/// directory without dictionaries.
///
/// The models of the dependency parser follow in a table of their own, when
/// there is one: the models embedded in the binary
/// (`embed-dependency-ipadic`, `embed-dependency-unidic` and the like) and
/// those `lindera download` installed, each with the value to pass as
/// `model` to the token filter `japanese_dependency` (#1440, #1441).
///
/// # Arguments
///
/// * `_args` - Parsed command line arguments (none).
///
/// # Returns
///
/// `Ok(())` when the listing is printed. An error when an embedded
/// dictionary cannot be loaded, when the dictionaries directory cannot be
/// determined or read, or when the listing cannot be written to stdout (see
/// `stdout_err`).
pub fn list(_args: ListArgs) -> LinderaResult<()> {
    let cli_version = get_version();
    let mut rows = Vec::new();
    for name in embedded_dictionary_names() {
        let dictionary = load_dictionary(&format!("embedded://{name}"))?;
        rows.push(embedded_row(
            name,
            dictionary.metadata.lindera_version.clone(),
            cli_version,
        ));
    }

    let dir = dictionaries_dir()?;
    let listing = directory_listing(&dir, cli_version)?;
    for skipped in &listing.skipped {
        write_line(
            Stream::Stderr,
            format_args!(
                "Skipped {}: not a dictionary (no metadata.json) and not a model (no model.json)",
                skipped.display()
            ),
        );
    }
    if listing.rows.is_empty() {
        write_line(
            Stream::Stderr,
            format_args!(
                "No dictionaries in {}; run 'lindera download <name>' to download one",
                dir.display()
            ),
        );
    }
    rows.extend(listing.rows);
    let mut models = embedded_model_rows();
    models.extend(listing.models);

    let mut output = render_rows(&HEADERS, &rows);
    if !models.is_empty() {
        output.push('\n');
        output.push_str(&render_rows(&MODEL_HEADERS, &models));
    }
    write_stdout(&output)
}

/// The rows of the models embedded in the binary.
///
/// # Returns
///
/// One row for each model a feature embeds, `embed-dependency-ipadic` for
/// IPADIC and `embed-dependency-<dictionary>` for a UniDic dictionary, with
/// `embedded://<dictionary>` as the value of `model`; no row for a build
/// without such a feature. An embedded model does not say which version of
/// Lindera built it, and is the one this binary was built with.
fn embedded_model_rows() -> Vec<DictionaryRow> {
    EMBEDDED_MODELS
        .iter()
        .filter(|(_, embedded)| *embedded)
        .map(|(dictionary, _)| DictionaryRow {
            name: format!("dependency-{dictionary}"),
            version: None,
            status: Status::Ok,
            dict: format!("embedded://{dictionary}"),
        })
        .collect()
}

/// The dictionaries a model can be embedded for, each with whether this
/// build embeds it.
const EMBEDDED_MODELS: [(&str, bool); 4] = [
    ("ipadic", cfg!(feature = "embed-dependency-ipadic")),
    ("unidic", cfg!(feature = "embed-dependency-unidic")),
    ("unidic-cwj", cfg!(feature = "embed-dependency-unidic-cwj")),
    ("unidic-csj", cfg!(feature = "embed-dependency-unidic-csj")),
];

/// Decides the state of a dictionary.
///
/// # Arguments
///
/// * `complete` - Whether all required files are present.
/// * `version` - Version of Lindera that built the dictionary, if it says.
/// * `cli_version` - Crate version of the running CLI.
///
/// # Returns
///
/// [`Status::Incomplete`] when files are missing, [`Status::Ok`] when the
/// CLI's version built it, and [`Status::OtherVersion`] otherwise.
fn status(complete: bool, version: Option<&str>, cli_version: &str) -> Status {
    if !complete {
        Status::Incomplete
    } else if version == Some(cli_version) {
        Status::Ok
    } else {
        Status::OtherVersion
    }
}

/// Builds the row of an embedded dictionary.
///
/// # Arguments
///
/// * `name` - Name of the embedded dictionary (e.g. `ipadic`).
/// * `version` - The `lindera_version` of its metadata.
/// * `cli_version` - Crate version of the running CLI.
///
/// # Returns
///
/// The row, with `embedded://<name>` as the value for `--dict`.
fn embedded_row(name: String, version: Option<String>, cli_version: &str) -> DictionaryRow {
    DictionaryRow {
        status: status(true, version.as_deref(), cli_version),
        dict: format!("embedded://{name}"),
        name,
        version,
    }
}

/// Collects the dictionaries in the dictionaries directory.
///
/// Every subdirectory with a `metadata.json` is a dictionary, complete or
/// not, and every other one with a `model.json` is a model of the
/// dependency parser. Entries whose name starts with `.` (the temporary
/// files of a download) and files are skipped silently; subdirectories
/// with neither file, such as the version directories of the layout before
/// v7.0.0, are returned as skipped.
///
/// # Arguments
///
/// * `dir` - The directory that holds the dictionaries.
/// * `cli_version` - Crate version of the running CLI.
///
/// # Returns
///
/// The rows and the skipped subdirectories, each by name; both empty when
/// `dir` does not exist. An `Io` error when `dir` cannot be read.
fn directory_listing(dir: &Path, cli_version: &str) -> LinderaResult<DirectoryListing> {
    if !dir.is_dir() {
        return Ok(DirectoryListing::default());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if !name.starts_with('.') && path.is_dir() {
            entries.push((name, path));
        }
    }
    entries.sort();

    let mut listing = DirectoryListing::default();
    for (name, path) in entries {
        if !path.join("metadata.json").is_file() {
            if path.join(MODEL_METADATA_FILE).is_file() {
                let kind = Downloadable::DependencyModel;
                let version = kind.installed_version(&path);
                listing.models.push(DictionaryRow {
                    status: status(kind.is_complete(&path), version.as_deref(), cli_version),
                    dict: kind.path_to_pass(&path).display().to_string(),
                    name,
                    version,
                });
                continue;
            }
            listing.skipped.push(path);
            continue;
        }
        let version = installed_lindera_version(&path);
        listing.rows.push(DictionaryRow {
            status: status(
                is_complete_dictionary_dir(&path),
                version.as_deref(),
                cli_version,
            ),
            dict: path.display().to_string(),
            name,
            version,
        });
    }
    Ok(listing)
}

/// Renders listing rows as a column-aligned table.
///
/// # Arguments
///
/// * `headers` - The header of each column.
/// * `rows` - Listing rows to render.
///
/// # Returns
///
/// The table as a string with a header line and one line per row, each
/// terminated by a newline. Columns are separated by two spaces; the
/// `VERSION` column shows [`UNKNOWN_VERSION`] when the dictionary does not
/// say.
fn render_rows(headers: &[&str; 4], rows: &[DictionaryRow]) -> String {
    let cells: Vec<[String; 4]> = rows
        .iter()
        .map(|row| {
            [
                row.name.clone(),
                row.version
                    .clone()
                    .unwrap_or_else(|| UNKNOWN_VERSION.to_string()),
                row.status.as_str().to_string(),
                row.dict.clone(),
            ]
        })
        .collect();

    // Width of each column except the last, which is left unpadded.
    let mut widths = [headers[0].len(), headers[1].len(), headers[2].len()];
    for row in &cells {
        for (width, cell) in widths.iter_mut().zip(row.iter()) {
            *width = (*width).max(cell.len());
        }
    }

    let mut output = String::new();
    render_line(&mut output, &headers.map(str::to_string), &widths);
    for row in &cells {
        render_line(&mut output, row, &widths);
    }
    output
}

/// Appends one table line to the output buffer.
///
/// # Arguments
///
/// * `output` - Buffer receiving the rendered line.
/// * `cells` - Cell values of the line.
/// * `widths` - Padded widths of all columns except the last.
fn render_line(output: &mut String, cells: &[String; 4], widths: &[usize; 3]) {
    for (cell, width) in cells.iter().zip(widths.iter()) {
        output.push_str(&format!("{cell:<width$}  "));
    }
    output.push_str(&cells[3]);
    output.push('\n');
}

#[cfg(test)]
mod tests {
    use lindera::dictionary::Metadata;

    use crate::dictionary_registry::REQUIRED_DICTIONARY_FILES;

    use super::*;

    /// Writes the required files into `dir`, creating it, with a
    /// `metadata.json` that says `version` built it (nothing when `None`).
    fn write_dictionary(dir: &Path, files: &[&str], version: Option<&str>) {
        fs::create_dir_all(dir).unwrap();
        for file in files {
            fs::write(dir.join(file), b"test").unwrap();
        }
        let metadata = Metadata {
            lindera_version: version.map(str::to_string),
            ..Metadata::default()
        };
        fs::write(
            dir.join("metadata.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn status_follows_the_files_and_the_version() {
        assert_eq!(status(true, Some("7.0.0"), "7.0.0"), Status::Ok);
        assert_eq!(status(true, Some("6.2.0"), "7.0.0"), Status::OtherVersion);
        assert_eq!(status(true, None, "7.0.0"), Status::OtherVersion);
        assert_eq!(status(false, Some("7.0.0"), "7.0.0"), Status::Incomplete);
    }

    #[test]
    fn embedded_row_names_the_embedded_uri() {
        assert_eq!(
            embedded_row("ipadic".to_string(), Some("7.0.0".to_string()), "7.0.0"),
            DictionaryRow {
                name: "ipadic".to_string(),
                version: Some("7.0.0".to_string()),
                status: Status::Ok,
                dict: "embedded://ipadic".to_string(),
            }
        );
    }

    #[test]
    fn missing_dictionaries_directory_lists_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let listing = directory_listing(&tmp.path().join("nonexistent"), "7.0.0").unwrap();
        assert_eq!(listing, DirectoryListing::default());
    }

    #[test]
    fn directory_listing_lists_dictionaries_by_name_and_skips_the_rest() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        write_dictionary(
            &dir.join("unidic"),
            &REQUIRED_DICTIONARY_FILES,
            Some("6.2.0"),
        );
        write_dictionary(
            &dir.join("ipadic"),
            &REQUIRED_DICTIONARY_FILES,
            Some("7.0.0"),
        );
        write_dictionary(&dir.join("mydic"), &["metadata.json"], Some("7.0.0"));
        write_dictionary(&dir.join("olddic"), &REQUIRED_DICTIONARY_FILES, None);
        // The version directory of the old layout, a temporary directory of
        // a download, and a file.
        fs::create_dir_all(dir.join("6.2.0").join("lindera-ipadic")).unwrap();
        write_dictionary(
            &dir.join(".tmp-lindera-ipadic-1.extract"),
            &REQUIRED_DICTIONARY_FILES,
            Some("7.0.0"),
        );
        fs::write(dir.join("notes.txt"), b"notes").unwrap();

        let listing = directory_listing(dir, "7.0.0").unwrap();
        let rows: Vec<(&str, Option<&str>, Status, String)> = listing
            .rows
            .iter()
            .map(|row| {
                (
                    row.name.as_str(),
                    row.version.as_deref(),
                    row.status,
                    row.dict.clone(),
                )
            })
            .collect();
        let path = |name: &str| dir.join(name).display().to_string();
        assert_eq!(
            rows,
            vec![
                ("ipadic", Some("7.0.0"), Status::Ok, path("ipadic")),
                ("mydic", Some("7.0.0"), Status::Incomplete, path("mydic")),
                ("olddic", None, Status::OtherVersion, path("olddic")),
                (
                    "unidic",
                    Some("6.2.0"),
                    Status::OtherVersion,
                    path("unidic")
                ),
            ]
        );
        assert_eq!(listing.skipped, vec![dir.join("6.2.0")]);
    }

    #[test]
    fn render_aligns_columns_and_marks_states() {
        let rows = vec![
            embedded_row("ipadic".to_string(), Some("7.0.0".to_string()), "7.0.0"),
            DictionaryRow {
                name: "mydic".to_string(),
                version: Some("7.0.0".to_string()),
                status: Status::Incomplete,
                dict: "/data/mydic".to_string(),
            },
            DictionaryRow {
                name: "olddic".to_string(),
                version: None,
                status: Status::OtherVersion,
                dict: "/data/olddic".to_string(),
            },
        ];

        let expected = "\
NAME    VERSION  STATUS         DICT
ipadic  7.0.0    ok             embedded://ipadic
mydic   7.0.0    incomplete     /data/mydic
olddic  unknown  other version  /data/olddic
";
        assert_eq!(render_rows(&HEADERS, &rows), expected);
    }

    /// Writes the files of a downloaded model into `dir`, creating it, with
    /// a `model.json` that says `version` built it (nothing when `None`).
    fn write_model(dir: &Path, files: &[&str], version: Option<&str>) {
        fs::create_dir_all(dir).unwrap();
        for file in files {
            fs::write(dir.join(file), b"test").unwrap();
        }
        let metadata = match version {
            Some(version) => serde_json::json!({
                "name": "dependency-ipadic",
                "dictionary": "ipadic",
                "lindera_version": version,
            }),
            None => serde_json::json!({ "name": "dependency-ipadic" }),
        };
        fs::write(
            dir.join("model.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn directory_listing_lists_the_models_apart_from_the_dictionaries() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        write_dictionary(
            &dir.join("ipadic"),
            &REQUIRED_DICTIONARY_FILES,
            Some("7.1.0"),
        );
        write_model(
            &dir.join("dependency-ipadic"),
            &["dependency.model", "NOTICE.txt"],
            Some("7.1.0"),
        );
        write_model(
            &dir.join("dependency-old"),
            &["dependency.model", "NOTICE.txt"],
            Some("7.0.0"),
        );
        write_model(
            &dir.join("dependency-partial"),
            &["NOTICE.txt"],
            Some("7.1.0"),
        );
        write_model(
            &dir.join("dependency-unknown"),
            &["dependency.model", "NOTICE.txt"],
            None,
        );
        fs::create_dir_all(dir.join("other")).unwrap();

        let listing = directory_listing(dir, "7.1.0").unwrap();
        let names: Vec<&str> = listing.rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["ipadic"]);
        let models: Vec<(&str, Option<&str>, Status, String)> = listing
            .models
            .iter()
            .map(|row| {
                (
                    row.name.as_str(),
                    row.version.as_deref(),
                    row.status,
                    row.dict.clone(),
                )
            })
            .collect();
        // The last cell is the model file, not its directory.
        let file = |name: &str| {
            dir.join(name)
                .join("dependency.model")
                .display()
                .to_string()
        };
        assert_eq!(
            models,
            vec![
                (
                    "dependency-ipadic",
                    Some("7.1.0"),
                    Status::Ok,
                    file("dependency-ipadic")
                ),
                (
                    "dependency-old",
                    Some("7.0.0"),
                    Status::OtherVersion,
                    file("dependency-old")
                ),
                (
                    "dependency-partial",
                    Some("7.1.0"),
                    Status::Incomplete,
                    file("dependency-partial")
                ),
                (
                    "dependency-unknown",
                    None,
                    Status::OtherVersion,
                    file("dependency-unknown")
                ),
            ]
        );
        assert_eq!(listing.skipped, vec![dir.join("other")]);
    }

    #[test]
    fn the_table_of_the_models_has_its_own_last_column() {
        let rows = vec![DictionaryRow {
            name: "dependency-ipadic".to_string(),
            version: Some("7.1.0".to_string()),
            status: Status::Ok,
            dict: "/data/dependency-ipadic/dependency.model".to_string(),
        }];
        let expected = "\
NAME               VERSION  STATUS  MODEL
dependency-ipadic  7.1.0    ok      /data/dependency-ipadic/dependency.model
";
        assert_eq!(render_rows(&MODEL_HEADERS, &rows), expected);
    }

    #[test]
    fn the_embedded_model_is_listed_in_a_build_that_has_one() {
        let rows = embedded_model_rows();
        let embedded: Vec<&str> = EMBEDDED_MODELS
            .iter()
            .filter(|(_, embedded)| *embedded)
            .map(|(dictionary, _)| *dictionary)
            .collect();
        assert_eq!(rows.len(), embedded.len());
        for (row, dictionary) in rows.iter().zip(embedded) {
            assert_eq!(row.name, format!("dependency-{dictionary}"));
            assert_eq!(row.dict, format!("embedded://{dictionary}"));
            assert_eq!(row.status, Status::Ok);
        }
        assert_eq!(
            rows.iter().any(|row| row.name == "dependency-ipadic"),
            cfg!(feature = "embed-dependency-ipadic")
        );
    }

    #[test]
    fn a_model_that_can_be_embedded_can_be_downloaded() {
        use crate::dictionary_registry::DOWNLOADABLE_MODELS;

        let names: Vec<String> = EMBEDDED_MODELS
            .iter()
            .map(|(dictionary, _)| format!("dependency-{dictionary}"))
            .collect();
        assert_eq!(names, DOWNLOADABLE_MODELS);
    }
}
