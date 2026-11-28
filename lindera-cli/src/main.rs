// `eprint!` and `eprintln!` panic when stderr has no reader or is full and
// non-blocking; the CLI's own messages go through `lindera_crf::console`,
// which waits while stderr is full and drops them when it has no reader
// (#1338, #1329). The messages of a download that fetches cannot be reached
// by the tests without the network, so clippy guards them.
#![deny(clippy::print_stderr)]

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use lindera_cli::{get_version, init_logger};

mod commands;
mod dictionary_registry;

use commands::build::BuildArgs;
#[cfg(feature = "dependency")]
use commands::dependency::DependencyArgs;
use commands::dictionary::{DictionaryArgs, DictionaryCommand};
use commands::download::DownloadArgs;
#[cfg(feature = "train")]
use commands::export::ExportArgs;
use commands::list::ListArgs;
use commands::tokenize::TokenizeArgs;
#[cfg(feature = "train")]
use commands::train::TrainArgs;

#[derive(Debug, Parser)]
#[clap(
    name = env!("CARGO_BIN_NAME"),
    author,
    about = "A morphological analysis command line interface",
    version = get_version(),
)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

// The subcommands of `lindera`: an operation of its own, `tokenize`, and
// one command for each object with operations, `dictionary` and
// `dependency` (#1482, #1483).
//
// The five hidden ones are the operations on dictionaries under the names
// they had as subcommands of `lindera` up to v7.0.0. They run what
// `lindera dictionary` runs, after a deprecation warning, and are removed
// in v8.0.0.
//
// Not a documentation comment: clap would take it for the help of the
// command.
#[derive(Debug, Subcommand)]
enum Commands {
    Tokenize(TokenizeArgs),
    Dictionary(DictionaryArgs),
    #[cfg(feature = "dependency")]
    Dependency(DependencyArgs),
    #[clap(hide = true)]
    List(ListArgs),
    #[clap(hide = true)]
    Download(DownloadArgs),
    #[clap(hide = true)]
    Build(BuildArgs),
    #[cfg(feature = "train")]
    #[clap(hide = true)]
    Train(TrainArgs),
    #[cfg(feature = "train")]
    #[clap(hide = true)]
    Export(ExportArgs),
}

/// Installs the logger, so that the warnings of the library crates reach
/// stderr, and runs the subcommand.
///
/// A subcommand whose standard output has no reader any more, as when it is
/// piped into `head`, ends quietly with exit code 0 (#1316). Any other error
/// is written to stderr as Rust writes the error returned from `main`, and
/// the exit code is 1.
///
/// # Returns
///
/// The exit code.
fn main() -> ExitCode {
    init_logger();
    let args = Args::parse();

    let result = match args.command {
        Commands::Tokenize(args) => commands::tokenize::tokenize(args),
        Commands::Dictionary(args) => commands::dictionary::dictionary(args),
        #[cfg(feature = "dependency")]
        Commands::Dependency(args) => commands::dependency::dependency(args),
        Commands::List(args) => commands::dictionary::deprecated(DictionaryCommand::List(args)),
        Commands::Download(args) => {
            commands::dictionary::deprecated(DictionaryCommand::Download(args))
        }
        Commands::Build(args) => commands::dictionary::deprecated(DictionaryCommand::Build(args)),
        #[cfg(feature = "train")]
        Commands::Train(args) => commands::dictionary::deprecated(DictionaryCommand::Train(args)),
        #[cfg(feature = "train")]
        Commands::Export(args) => commands::dictionary::deprecated(DictionaryCommand::Export(args)),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if commands::is_stdout_closed(&err) => ExitCode::SUCCESS,
        Err(err) => {
            // A failure to write the error is ignored, as Rust's default for
            // `main` does.
            let _ = writeln!(io::stderr(), "Error: {err:?}");
            ExitCode::FAILURE
        }
    }
}
