use clap::{Parser, Subcommand};

use lindera::LinderaResult;
use lindera_cli::get_version;

mod commands;

use commands::build::BuildArgs;
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
    #[cfg(feature = "train")]
    Train(TrainArgs),
    #[cfg(feature = "train")]
    Export(ExportArgs),
}

fn main() -> LinderaResult<()> {
    let args = Args::parse();

    match args.command {
        Commands::List(args) => commands::list::list(args),
        Commands::Tokenize(args) => commands::tokenize::tokenize(args),
        Commands::Build(args) => commands::build::build(args),
        #[cfg(feature = "train")]
        Commands::Train(args) => commands::train::train(args),
        #[cfg(feature = "train")]
        Commands::Export(args) => commands::export::export(args),
    }
}
