//! `lindera dependency download` and `lindera dependency list`: the models
//! of the dependency parser a release publishes, one per dictionary.
//!
//! A model is downloaded and installed as a dictionary is, next to the
//! dictionaries, and the two subcommands run the code of `lindera download`
//! and `lindera list`. They are apart from those two so that a model is
//! named by its dictionary, and so that the operations on models are with
//! the other operations of the dependency analysis (#1482).

use lindera::LinderaResult;
use lindera_cli::get_version;

use crate::commands::{download, list};
use crate::dictionary_registry::{MODEL_DICTIONARIES, MODEL_NAME_PREFIX};

/// The arguments of `download`.
#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Download the model of the dependency parser for a dictionary from the Lindera GitHub releases",
    version = get_version(),
)]
pub(super) struct DownloadArgs {
    /// The dictionary the model is for.
    #[clap(
        value_parser = clap::builder::PossibleValuesParser::new(MODEL_DICTIONARIES),
        help = "Name of the dictionary the model is for"
    )]
    dictionary: String,
    /// Replaces an already downloaded model when set.
    #[clap(long = "force", help = "Re-download and replace an existing model")]
    force: bool,
}

/// The arguments of `list`.
#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "List the models of the dependency parser embedded in the binary and those in LINDERA_DICTIONARIES_DIR",
    version = get_version(),
)]
pub(super) struct ListArgs {}

/// Runs `download`.
///
/// Downloads the archive of the model for the dictionary, of the CLI's own
/// version, from the GitHub releases page, and installs it in
/// `<LINDERA_DICTIONARIES_DIR>/dependency-<dictionary>`. The path of the
/// model file, the value to pass to `--model` of `lindera dependency parse`
/// and as `model` to the token filter `japanese_dependency`, is printed to
/// stdout; progress and notices go to stderr.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` when the model is installed, or already in place, and the path
/// of its file is printed. An error when the download or the installation
/// fails, or when the path cannot be written to stdout.
pub(super) fn download(args: DownloadArgs) -> LinderaResult<()> {
    let name = format!("{MODEL_NAME_PREFIX}{}", args.dictionary);
    download::download_named(&name, &args.dictionary, args.force)
}

/// Runs `list`.
///
/// # Arguments
///
/// * `_args` - The arguments (none).
///
/// # Returns
///
/// `Ok(())` when the table of the models is printed: see
/// `list::list_models`.
pub(super) fn list(_args: ListArgs) -> LinderaResult<()> {
    list::list_models()
}
