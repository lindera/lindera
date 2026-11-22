//! The `lindera-dependency` command: the tools around the dependency parser
//! that need a tokenizer.
//!
//! `convert` makes training data from a dependency corpus:
//!
//! ```text
//! lindera-dependency convert --dict /path/to/ipadic \
//!     --input ja_gsd-ud-train.conllu --output train.lattice
//! ```
//!
//! The corpus is tokenized with Lindera and the dictionary given, with the
//! default settings, as the parser will be used; the result is written in
//! the lattice format, and a report of what was kept, merged and dropped
//! goes to the standard error.

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use lindera_dependency::corpus::{self, DictionaryMorpheme, TokenizerError};
use lindera_segmenter::dictionary::load_dictionary;
use lindera_segmenter::mode::Mode;
use lindera_segmenter::segmenter::Segmenter;

/// The arguments of the command.
#[derive(Debug, Parser)]
#[clap(
    name = "lindera-dependency",
    author,
    about = "Tools for the bunsetsu dependency parser of Lindera",
    version
)]
struct Args {
    /// The subcommand.
    #[clap(subcommand)]
    command: Command,
}

/// The subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Makes training data in the lattice format from a dependency corpus.
    Convert(ConvertArgs),
}

/// The arguments of `convert`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Make training data in the lattice format from a CoNLL-U file with bunsetsu labels (UD Japanese-GSD, UD Japanese-PUD)"
)]
struct ConvertArgs {
    /// The dictionary the corpus is tokenized with.
    #[clap(
        short = 'd',
        long = "dict",
        help = "Dictionary URI or directory path (e.g. /path/to/dictionary, or embedded://ipadic in a build with embed-ipadic)"
    )]
    dict: String,
    /// The corpus.
    #[clap(short = 'i', long = "input", help = "CoNLL-U file to convert")]
    input: PathBuf,
    /// The training data.
    #[clap(
        short = 'o',
        long = "output",
        help = "File to write, in the lattice format"
    )]
    output: PathBuf,
}

/// The error type of the command.
type CommandError = Box<dyn std::error::Error + Send + Sync>;

/// Runs `convert`.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the output and the report are written, or the first error:
/// a dictionary that cannot be loaded, a file that cannot be read or
/// written, or an input that cannot be converted.
fn convert(args: ConvertArgs) -> Result<(), CommandError> {
    let dictionary = load_dictionary(&args.dict)
        .map_err(|error| format!("cannot load the dictionary {}: {error}", args.dict))?;
    // The default settings: the data has to be tokenized as the text the
    // parser will be used on.
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenize = |text: &str| -> Result<Vec<DictionaryMorpheme>, TokenizerError> {
        let mut tokens = segmenter.segment(Cow::Borrowed(text))?;
        Ok(tokens
            .iter_mut()
            .map(|token| {
                let features = token.details_iter().map(str::to_string).collect();
                DictionaryMorpheme::new(token.surface.to_string(), features)
            })
            .collect())
    };

    let input = File::open(&args.input)
        .map_err(|error| format!("cannot open {}: {error}", args.input.display()))?;
    let output = File::create(&args.output)
        .map_err(|error| format!("cannot create {}: {error}", args.output.display()))?;
    let mut output = BufWriter::new(output);
    let report = corpus::convert(BufReader::new(input), &mut output, tokenize)
        .map_err(|error| format!("{}: {error}", args.input.display()))?;
    output
        .flush()
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;

    // A failure to write the report is ignored: the output is complete.
    let _ = writeln!(io::stderr(), "{}\n{report}", args.input.display());
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    let result = match args.command {
        Command::Convert(args) => convert(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // A failure to write the error is ignored, as Rust's default for
            // `main` does.
            let _ = writeln!(io::stderr(), "Error: {error}");
            ExitCode::FAILURE
        }
    }
}
