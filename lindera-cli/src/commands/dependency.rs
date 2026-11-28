//! The `lindera dependency` command: the bunsetsu dependency analysis of
//! `lindera-dependency`, from the command line.
//!
//! `parse` reads plain text, one sentence per line, tokenizes it with a
//! Lindera dictionary, and writes the bunsetsu of each sentence and their
//! dependencies in the lattice format, which is what `cabocha -f1` writes:
//!
//! ```text
//! lindera dependency parse --model ipadic.model --dict /path/to/ipadic < text.txt
//! ```
//!
//! With `--input-layer` it takes input that is tokenized already
//! (`morphemes`), or segmented into bunsetsu as well (`bunsetsu`), in the
//! lattice format. `chunk` segments the morphemes of a file into bunsetsu
//! and does not parse them.
//!
//! These two are in a build with the `dependency` feature, which is in the
//! default build. With the `train` feature as well, the command also makes
//! training data (`convert`, `instances`, `split`), trains the chunker and
//! the parser (`train-chunker`, `train-parser`), converts a model of CaboCha
//! (`convert-cabocha`) and evaluates (`evaluate`):
//!
//! ```text
//! lindera dependency convert --dict /path/to/ipadic \
//!     --input ja_gsd-ud-train.conllu --output train.lattice
//! lindera dependency train-chunker --input train.lattice --output chunker.model
//! lindera dependency train-parser --input train.lattice --chunker chunker.model \
//!     --output ipadic.model
//! lindera dependency parse --model ipadic.model --input-layer bunsetsu \
//!     --input dev.lattice --output output.lattice
//! lindera dependency evaluate --gold dev.lattice --system output.lattice
//! ```
//!
//! The command was the binary of the `lindera-dependency` crate before
//! 7.1.0 was released (#1482); the crate is a library only.

mod models;
mod parse;
#[cfg(feature = "train")]
mod train;

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use clap::Subcommand;
use lindera::LinderaResult;
use lindera::error::{LinderaError, LinderaErrorKind};
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::{Sentence, embedded};

use lindera_cli::get_version;

/// The arguments of `lindera dependency`.
#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Analyze bunsetsu dependencies: parse text, and with the train feature make training data, train and evaluate",
    version = get_version(),
)]
pub struct DependencyArgs {
    /// The subcommand.
    #[clap(subcommand)]
    command: Command,
}

/// The subcommands of `lindera dependency`.
#[derive(Debug, Subcommand)]
enum Command {
    /// Finds the bunsetsu of each sentence and their dependencies.
    Parse(parse::ParseArgs),
    /// Segments the morphemes of a file into bunsetsu with a model.
    Chunk(parse::ChunkArgs),
    /// Downloads the model for a dictionary.
    Download(models::DownloadArgs),
    /// Lists the models in place and those embedded in the binary.
    List(models::ListArgs),
    /// Makes training data in the lattice format from a dependency corpus.
    #[cfg(feature = "train")]
    Convert(train::ConvertArgs),
    /// Evaluates an output against the answer.
    #[cfg(feature = "train")]
    Evaluate(train::EvaluateArgs),
    /// Splits a file into the folds of a cross-validation.
    #[cfg(feature = "train")]
    Split(train::SplitArgs),
    /// Trains the model of the chunker.
    #[cfg(feature = "train")]
    TrainChunker(train::TrainChunkerArgs),
    /// Writes the training instances of the parser.
    #[cfg(feature = "train")]
    Instances(train::InstancesArgs),
    /// Trains the parser and writes a model file.
    #[cfg(feature = "train")]
    TrainParser(train::TrainParserArgs),
    /// Converts a model of CaboCha into a model file.
    #[cfg(feature = "train")]
    ConvertCabocha(train::ConvertCabochaArgs),
}

/// The error type inside the subcommands: any error, or a message.
///
/// The subcommands name the file or the line in a message of their own, and
/// take the errors of the library with `?`. [`dependency`] turns what they
/// return into a `LinderaError`, which is what every command of `lindera`
/// returns.
type CommandError = Box<dyn std::error::Error + Send + Sync>;

/// Turns the error of a subcommand into a `LinderaError`.
///
/// A `LinderaError` that went through a subcommand, such as the one of a
/// write to a standard output without a reader, comes back as it was, so
/// that `main` still recognizes it. Any other error becomes the source of an
/// error of the `Content` kind, with its message unchanged.
///
/// # Arguments
///
/// * `error` - The error a subcommand returned.
///
/// # Returns
///
/// The error for `main`.
fn into_lindera_error(error: CommandError) -> LinderaError {
    match error.downcast::<LinderaError>() {
        Ok(error) => *error,
        Err(error) => LinderaErrorKind::Content.with_error(anyhow::anyhow!(error)),
    }
}

/// Reads the sentences of a file in the lattice format.
///
/// # Arguments
///
/// * `path` - The file.
///
/// # Returns
///
/// The sentences, or an error that names the file.
fn read_lattice(path: &Path) -> Result<Vec<Sentence<LatticeMorpheme>>, CommandError> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    lattice::Reader::new(BufReader::new(file))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", path.display()).into())
}

/// Reads the bytes of a model: of the build, or of a file.
///
/// # Arguments
///
/// * `model` - What `--model` was given: `embedded://<dictionary>`, or a
///   path.
///
/// # Returns
///
/// The bytes embedded in the build for the dictionary, without a copy, or
/// the content of the file; or an error for a build that embeds no such
/// model, and for a file that cannot be read.
fn read_model(model: &str) -> Result<Cow<'static, [u8]>, CommandError> {
    match embedded::dictionary_of(model) {
        Some(dictionary) => Ok(Cow::Borrowed(
            embedded::bytes(dictionary).map_err(|error| format!("{model}: {error}"))?,
        )),
        None => {
            Ok(Cow::Owned(fs::read(model).map_err(|error| {
                format!("cannot read {model}: {error}")
            })?))
        }
    }
}

/// Runs `lindera dependency`.
///
/// # Arguments
///
/// * `args` - The arguments, with the subcommand.
///
/// # Returns
///
/// `Ok(())` once the subcommand has written its output, or its first error.
pub fn dependency(args: DependencyArgs) -> LinderaResult<()> {
    let result = match args.command {
        // The two that share their code with `lindera download` and
        // `lindera list` return a `LinderaError` already.
        Command::Download(args) => return models::download(args),
        Command::List(args) => return models::list(args),
        Command::Parse(args) => parse::parse(args),
        Command::Chunk(args) => parse::chunk(args),
        #[cfg(feature = "train")]
        Command::Convert(args) => train::convert(args),
        #[cfg(feature = "train")]
        Command::Evaluate(args) => train::evaluate(args),
        #[cfg(feature = "train")]
        Command::Split(args) => train::split(args),
        #[cfg(feature = "train")]
        Command::TrainChunker(args) => train::train_chunker(args),
        #[cfg(feature = "train")]
        Command::Instances(args) => train::instances(args),
        #[cfg(feature = "train")]
        Command::TrainParser(args) => train::train_parser(args),
        #[cfg(feature = "train")]
        Command::ConvertCabocha(args) => train::convert_cabocha(args),
    };
    result.map_err(into_lindera_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{is_stdout_closed, stdout_err};

    #[test]
    fn a_message_becomes_the_source_of_a_content_error() {
        let error = into_lindera_error("cannot open a.lattice: not found".into());
        assert_eq!(error.kind(), LinderaErrorKind::Content);
        assert_eq!(
            format!("{error:?}"),
            "LinderaError { kind: Content, source: cannot open a.lattice: not found }"
        );
        assert!(!is_stdout_closed(&error));
    }

    #[test]
    fn a_closed_standard_output_is_still_recognized() {
        let closed = stdout_err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        let error = into_lindera_error(Box::new(closed));
        assert!(is_stdout_closed(&error));
    }
}
