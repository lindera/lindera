// `eprint!` and `eprintln!` panic when stderr has no reader; the CLI's own
// messages go through `lindera_cli::write_stderr` and `write_stderr_line`,
// which drop them instead (#1338). The messages of a download that fetches
// cannot be reached by the tests without the network, so clippy guards them.
#![deny(clippy::print_stderr)]

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use lindera_cli::{get_version, init_logger};

mod commands;
mod dictionary_registry;

use commands::build::BuildArgs;
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

#[derive(Debug, Subcommand)]
enum Commands {
    List(ListArgs),
    Tokenize(TokenizeArgs),
    Build(BuildArgs),
    Download(DownloadArgs),
    #[cfg(feature = "train")]
    Train(TrainArgs),
    #[cfg(feature = "train")]
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
        Commands::List(args) => commands::list::list(args),
        Commands::Tokenize(args) => commands::tokenize::tokenize(args),
        Commands::Build(args) => commands::build::build(args),
        Commands::Download(args) => commands::download::download(args),
        #[cfg(feature = "train")]
        Commands::Train(args) => commands::train::train(args),
        #[cfg(feature = "train")]
        Commands::Export(args) => commands::export::export(args),
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
