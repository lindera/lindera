use std::path::{Path, PathBuf};

use lindera::LinderaResult;
use lindera::dictionary::DictionaryKind;
use lindera_cli::get_version;

use crate::dictionary_registry::{
    DATA_DIR_ENV, DOWNLOADABLE_DICTIONARIES, base_data_dir, dictionary_dir,
    is_complete_dictionary_dir,
};

/// Column headers of the dictionary listing table.
const HEADERS: [&str; 4] = ["NAME", "EMBEDDED", "DOWNLOADED", "PATH"];

/// Placeholder shown in the `PATH` column when no installation directory
/// exists.
const NO_PATH: &str = "-";

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "List morphological analysis dictionaries and their status",
    version = get_version(),
)]
/// Arguments for the `list` subcommand.
pub struct ListArgs {}

/// Local download state of a dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DownloadState {
    /// No installation directory exists.
    NotDownloaded,
    /// A complete installation exists at the contained path.
    Downloaded(PathBuf),
    /// An installation directory exists but required files are missing.
    Incomplete(PathBuf),
}

/// One row of the dictionary listing.
#[derive(Debug)]
struct DictionaryRow {
    /// Dictionary name (e.g. `ipadic`).
    name: String,
    /// Whether the dictionary is embedded in the binary.
    embedded: bool,
    /// Local download state of the dictionary.
    download: DownloadState,
}

/// Runs the `list` subcommand.
///
/// Prints a table of all known dictionaries showing whether each one is
/// embedded in the binary (via `embed-*` cargo features) and whether a
/// pre-built copy is downloaded locally (see `lindera download`).
///
/// # Arguments
///
/// * `_args` - Parsed command line arguments (none).
///
/// # Returns
///
/// `Ok(())` when the listing is printed. An `Io` error when the application
/// data directory cannot be determined.
pub fn list(_args: ListArgs) -> LinderaResult<()> {
    let embedded: Vec<String> = DictionaryKind::contained_variants()
        .iter()
        .map(|kind| kind.as_str().to_string())
        .collect();
    let base = base_data_dir(std::env::var_os(DATA_DIR_ENV).map(PathBuf::from))?;
    let rows = dictionary_rows(&embedded, &base, get_version());
    print!("{}", render_rows(&rows));
    Ok(())
}

/// Collects the listing rows for all known dictionaries.
///
/// The listed set is the union of [`DOWNLOADABLE_DICTIONARIES`] and the
/// embedded dictionary names. Download detection only considers the given
/// CLI version's installation directory, matching how `tokenize --dict`
/// resolves dictionary names.
///
/// # Arguments
///
/// * `embedded` - Names of the dictionaries embedded in the binary.
/// * `base` - Base application data directory (see `base_data_dir`).
/// * `version` - Crate version of the running CLI.
///
/// # Returns
///
/// One row per known dictionary, in [`DOWNLOADABLE_DICTIONARIES`] order with
/// any extra embedded names appended.
fn dictionary_rows(embedded: &[String], base: &Path, version: &str) -> Vec<DictionaryRow> {
    let mut names: Vec<String> = DOWNLOADABLE_DICTIONARIES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    for name in embedded {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }

    names
        .into_iter()
        .map(|name| {
            let dir = dictionary_dir(base, version, &name);
            let download = if is_complete_dictionary_dir(&dir) {
                DownloadState::Downloaded(dir)
            } else if dir.is_dir() {
                DownloadState::Incomplete(dir)
            } else {
                DownloadState::NotDownloaded
            };
            DictionaryRow {
                embedded: embedded.contains(&name),
                name,
                download,
            }
        })
        .collect()
}

/// Renders listing rows as a column-aligned table.
///
/// # Arguments
///
/// * `rows` - Listing rows to render.
///
/// # Returns
///
/// The table as a string with a header line and one line per row, each
/// terminated by a newline. Columns are separated by two spaces; the `PATH`
/// column shows [`NO_PATH`] when no installation directory exists.
fn render_rows(rows: &[DictionaryRow]) -> String {
    let cells: Vec<[String; 4]> = rows
        .iter()
        .map(|row| {
            let (downloaded, path) = match &row.download {
                DownloadState::NotDownloaded => ("no".to_string(), NO_PATH.to_string()),
                DownloadState::Downloaded(dir) => ("yes".to_string(), dir.display().to_string()),
                DownloadState::Incomplete(dir) => {
                    ("incomplete".to_string(), dir.display().to_string())
                }
            };
            let embedded = if row.embedded { "yes" } else { "no" };
            [row.name.clone(), embedded.to_string(), downloaded, path]
        })
        .collect();

    // Width of each column except the last, which is left unpadded.
    let mut widths = [HEADERS[0].len(), HEADERS[1].len(), HEADERS[2].len()];
    for row in &cells {
        for (width, cell) in widths.iter_mut().zip(row.iter()) {
            *width = (*width).max(cell.len());
        }
    }

    let mut output = String::new();
    render_line(&mut output, &HEADERS.map(str::to_string), &widths);
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
    use std::fs;

    use crate::dictionary_registry::REQUIRED_DICTIONARY_FILES;

    use super::*;

    /// Creates the given files (with dummy content) inside a directory.
    fn create_files(dir: &Path, files: &[&str]) {
        fs::create_dir_all(dir).unwrap();
        for file in files {
            fs::write(dir.join(file), b"test").unwrap();
        }
    }

    /// Returns the row with the given name, panicking when absent.
    fn row_by_name<'a>(rows: &'a [DictionaryRow], name: &str) -> &'a DictionaryRow {
        rows.iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("row {name} not found"))
    }

    #[test]
    fn rows_cover_all_downloadable_dictionaries() {
        let tmp = tempfile::tempdir().unwrap();
        let rows = dictionary_rows(&[], tmp.path(), "5.1.0");
        let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, DOWNLOADABLE_DICTIONARIES.to_vec());
        assert!(
            rows.iter()
                .all(|row| !row.embedded && row.download == DownloadState::NotDownloaded)
        );
    }

    #[test]
    fn complete_installation_is_reported_with_path() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = dictionary_dir(tmp.path(), "5.1.0", "ipadic");
        create_files(&dir, &REQUIRED_DICTIONARY_FILES);

        let rows = dictionary_rows(&[], tmp.path(), "5.1.0");
        assert_eq!(
            row_by_name(&rows, "ipadic").download,
            DownloadState::Downloaded(dir)
        );
    }

    #[test]
    fn incomplete_installation_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = dictionary_dir(tmp.path(), "5.1.0", "unidic");
        create_files(&dir, &["metadata.json"]);

        let rows = dictionary_rows(&[], tmp.path(), "5.1.0");
        assert_eq!(
            row_by_name(&rows, "unidic").download,
            DownloadState::Incomplete(dir)
        );
    }

    #[test]
    fn other_version_installation_is_not_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = dictionary_dir(tmp.path(), "5.0.0", "ipadic");
        create_files(&dir, &REQUIRED_DICTIONARY_FILES);

        let rows = dictionary_rows(&[], tmp.path(), "5.1.0");
        assert_eq!(
            row_by_name(&rows, "ipadic").download,
            DownloadState::NotDownloaded
        );
    }

    #[test]
    fn embedded_flag_follows_embedded_names() {
        let tmp = tempfile::tempdir().unwrap();
        let embedded = vec!["ipadic".to_string()];
        let rows = dictionary_rows(&embedded, tmp.path(), "5.1.0");
        assert!(row_by_name(&rows, "ipadic").embedded);
        assert!(!row_by_name(&rows, "unidic").embedded);
    }

    #[test]
    fn embedded_name_outside_downloadable_set_is_appended() {
        let tmp = tempfile::tempdir().unwrap();
        let embedded = vec!["custom-dic".to_string()];
        let rows = dictionary_rows(&embedded, tmp.path(), "5.1.0");
        let row = row_by_name(&rows, "custom-dic");
        assert!(row.embedded);
        assert_eq!(rows.len(), DOWNLOADABLE_DICTIONARIES.len() + 1);
    }

    #[test]
    fn render_aligns_columns_and_marks_states() {
        let rows = vec![
            DictionaryRow {
                name: "ipadic".to_string(),
                embedded: true,
                download: DownloadState::Downloaded(PathBuf::from("/data/lindera-ipadic")),
            },
            DictionaryRow {
                name: "ipadic-neologd".to_string(),
                embedded: false,
                download: DownloadState::NotDownloaded,
            },
            DictionaryRow {
                name: "unidic".to_string(),
                embedded: false,
                download: DownloadState::Incomplete(PathBuf::from("/data/lindera-unidic")),
            },
        ];

        let expected = "\
NAME            EMBEDDED  DOWNLOADED  PATH
ipadic          yes       yes         /data/lindera-ipadic
ipadic-neologd  no        no          -
unidic          no        incomplete  /data/lindera-unidic
";
        assert_eq!(render_rows(&rows), expected);
    }
}
