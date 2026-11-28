//! The `lindera dictionary` command: the operations on dictionaries.
//!
//! ```text
//! lindera dictionary list
//! lindera dictionary download ipadic
//! lindera dictionary build --src <dir> --dest <dir> --metadata <file>
//! lindera dictionary train ...     (with the `train` feature)
//! lindera dictionary export ...    (with the `train` feature)
//! ```
//!
//! The five were subcommands of `lindera` itself up to v7.0.0: `lindera
//! list`, `lindera download`, `lindera build`, `lindera train` and `lindera
//! export`. Those names still run, hidden from the help, and write a
//! deprecation warning first; they are removed in v8.0.0 (#1483). The
//! command line of Lindera has the object and then the operation:
//! `lindera dictionary` for dictionaries, and `lindera dependency` for the
//! dependency analysis (#1482).

use clap::Subcommand;
use lindera::LinderaResult;
use lindera_cli::get_version;
use lindera_crf::console::{Stream, write_line};

use super::build::{self, BuildArgs};
use super::download::{self, DownloadArgs};
#[cfg(feature = "train")]
use super::export::{self, ExportArgs};
use super::list::{self, ListArgs};
#[cfg(feature = "train")]
use super::train::{self, TrainArgs};

/// The arguments of `lindera dictionary`.
#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Work with dictionaries: list, download and build them, and with the train feature train and export them",
    version = get_version(),
)]
pub struct DictionaryArgs {
    /// The subcommand.
    #[clap(subcommand)]
    command: DictionaryCommand,
}

/// An operation on dictionaries: a subcommand of `lindera dictionary`, and
/// what a deprecated subcommand of `lindera` of the same name runs.
///
/// The variants have no documentation comment of their own: the help of a
/// subcommand is the `about` of its arguments, as it was for the
/// subcommands of `lindera`.
#[derive(Debug, Subcommand)]
pub enum DictionaryCommand {
    List(ListArgs),
    Download(DownloadArgs),
    Build(BuildArgs),
    #[cfg(feature = "train")]
    Train(TrainArgs),
    #[cfg(feature = "train")]
    Export(ExportArgs),
}

impl DictionaryCommand {
    /// The name of the operation, as it is typed after `lindera dictionary`.
    ///
    /// # Returns
    ///
    /// `list`, `download`, `build`, `train` or `export`.
    fn name(&self) -> &'static str {
        match self {
            DictionaryCommand::List(_) => "list",
            DictionaryCommand::Download(_) => "download",
            DictionaryCommand::Build(_) => "build",
            #[cfg(feature = "train")]
            DictionaryCommand::Train(_) => "train",
            #[cfg(feature = "train")]
            DictionaryCommand::Export(_) => "export",
        }
    }

    /// Runs the operation.
    ///
    /// # Returns
    ///
    /// What the function of the operation returns.
    fn run(self) -> LinderaResult<()> {
        match self {
            DictionaryCommand::List(args) => list::list(args),
            DictionaryCommand::Download(args) => download::download(args),
            DictionaryCommand::Build(args) => build::build(args),
            #[cfg(feature = "train")]
            DictionaryCommand::Train(args) => train::train(args),
            #[cfg(feature = "train")]
            DictionaryCommand::Export(args) => export::export(args),
        }
    }
}

/// The warning a deprecated subcommand of `lindera` writes: the form of the
/// warnings about `LINDERA_DATA_DIR` and `LINDERA_CONFIG_PATH` (#1303,
/// #1304).
///
/// # Arguments
///
/// * `name` - The name of the operation, such as `build`.
///
/// # Returns
///
/// The line, without a line break.
fn deprecation_warning(name: &str) -> String {
    format!(
        "Warning: `lindera {name}` is deprecated and will be removed in v8.0.0; use `lindera dictionary {name}` instead"
    )
}

/// Runs `lindera dictionary`.
///
/// # Arguments
///
/// * `args` - The arguments, with the subcommand.
///
/// # Returns
///
/// `Ok(())` once the subcommand has written its output, or its error.
pub fn dictionary(args: DictionaryArgs) -> LinderaResult<()> {
    args.command.run()
}

/// Runs an operation that was typed as a subcommand of `lindera` itself, the
/// form up to v7.0.0: `lindera build` for `lindera dictionary build`.
///
/// Writes the deprecation warning to stderr, then runs the operation as
/// `lindera dictionary` does: the standard output and the result are the
/// same. The warning goes through `write_line`, which drops it when stderr
/// has no reader and waits while a non-blocking stderr is full (#1338,
/// #1329).
///
/// # Arguments
///
/// * `command` - The operation.
///
/// # Returns
///
/// What the operation returns.
pub fn deprecated(command: DictionaryCommand) -> LinderaResult<()> {
    write_line(
        Stream::Stderr,
        format_args!("{}", deprecation_warning(command.name())),
    );
    command.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_warning_names_the_old_form_and_the_new_one() {
        assert_eq!(
            deprecation_warning("build"),
            "Warning: `lindera build` is deprecated and will be removed in v8.0.0; use `lindera dictionary build` instead"
        );
    }
}
