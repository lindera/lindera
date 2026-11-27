//! The `lindera-dependency` command.
//!
//! `parse` reads plain text, one sentence per line, tokenizes it with a
//! Lindera dictionary, and writes the bunsetsu of each sentence and their
//! dependencies in the lattice format, which is what `cabocha -f1` writes:
//!
//! ```text
//! lindera-dependency parse --model ipadic.model --dict /path/to/ipadic < text.txt
//! ```
//!
//! With `--input-layer` it takes input that is tokenized already
//! (`morphemes`), or segmented into bunsetsu as well (`bunsetsu`), in the
//! lattice format. `chunk` segments the morphemes of a file into bunsetsu
//! and does not parse them.
//!
//! These two are in the build with the `cli` feature, which has no training
//! code. With the `train` feature the command also makes training data
//! (`convert`, `instances`, `split`), trains the chunker and the parser
//! (`train-chunker`, `train-parser`) and evaluates (`evaluate`):
//!
//! ```text
//! lindera-dependency convert --dict /path/to/ipadic \
//!     --input ja_gsd-ud-train.conllu --output train.lattice
//! lindera-dependency train-chunker --input train.lattice --output chunker.model
//! lindera-dependency train-parser --input train.lattice --chunker chunker.model \
//!     --output ipadic.model
//! lindera-dependency parse --model ipadic.model --input-layer bunsetsu \
//!     --input dev.lattice --output output.lattice
//! lindera-dependency evaluate --gold dev.lattice --system output.lattice
//! ```

#[cfg(feature = "train")]
#[path = "lindera-dependency/train.rs"]
mod train;

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser as _, Subcommand};
use lindera_dependency::chunker::{Chunker, ChunkerData};
use lindera_dependency::embedded;
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::Model;
use lindera_dependency::parser::{Classifier, Parser};
use lindera_dependency::{Bunsetsu, DictionaryMorpheme, Morpheme, Sentence};
use lindera_segmenter::dictionary::load_dictionary;
use lindera_segmenter::mode::Mode;
use lindera_segmenter::segmenter::Segmenter;

/// The arguments of the command.
#[derive(Debug, clap::Parser)]
#[clap(
    name = "lindera-dependency",
    author,
    about = "The bunsetsu dependency parser of Lindera",
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
    /// Finds the bunsetsu of each sentence and their dependencies.
    Parse(ParseArgs),
    /// Segments the morphemes of a file into bunsetsu with a model.
    Chunk(ChunkArgs),
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

/// What the input of `parse` holds already.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum InputLayer {
    /// Plain text, one sentence per line: it is tokenized, segmented and
    /// parsed.
    Text,
    /// Morphemes in the lattice format: they are segmented and parsed, and
    /// bunsetsu of the input are not read.
    Morphemes,
    /// Bunsetsu in the lattice format: they are parsed.
    Bunsetsu,
}

/// The arguments of `parse`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Read sentences, one per line, tokenize them with a dictionary, and write their bunsetsu and dependencies in the lattice format (what `cabocha -f1` writes); or take input that is tokenized or segmented already"
)]
struct ParseArgs {
    /// The model.
    #[clap(
        short = 'm',
        long = "model",
        help = "Model file with a parser, and a chunker unless the input has its bunsetsu (what train-parser --chunker writes), or embedded://<dictionary> in a build with embed-model-<dictionary> (ipadic, unidic, unidic-cwj, unidic-csj)"
    )]
    model: String,
    /// The dictionary the text is tokenized with.
    #[clap(
        short = 'd',
        long = "dict",
        help = "Dictionary URI or directory path, for the input layer text: the dictionary the model was trained for (e.g. /path/to/dictionary, or embedded://ipadic in a build with embed-ipadic)"
    )]
    dict: Option<String>,
    /// What the input holds already.
    #[clap(
        short = 'I',
        long = "input-layer",
        value_enum,
        default_value = "text",
        help = "What the input holds: text (one sentence per line), morphemes or bunsetsu (both in the lattice format)"
    )]
    input_layer: InputLayer,
    /// The sentences.
    #[clap(
        short = 'i',
        long = "input",
        help = "File to parse (default: the standard input)"
    )]
    input: Option<PathBuf>,
    /// The parsed sentences.
    #[clap(
        short = 'o',
        long = "output",
        help = "File to write, in the lattice format (default: the standard output)"
    )]
    output: Option<PathBuf>,
}

/// The arguments of `chunk`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Segment the morphemes of a file in the lattice format into bunsetsu with a chunker model; the bunsetsu of the input are not read, and each bunsetsu written depends on the next"
)]
struct ChunkArgs {
    /// The model.
    #[clap(
        short = 'm',
        long = "model",
        help = "Model file that train-chunker wrote, or embedded://<dictionary> in a build with embed-model-<dictionary> (ipadic, unidic, unidic-cwj, unidic-csj)"
    )]
    model: String,
    /// The morphemes.
    #[clap(
        short = 'i',
        long = "input",
        help = "File to segment, in the lattice format"
    )]
    input: PathBuf,
    /// The segmented sentences.
    #[clap(
        short = 'o',
        long = "output",
        help = "File to write, in the lattice format"
    )]
    output: PathBuf,
}

/// The error type of the command.
type CommandError = Box<dyn std::error::Error + Send + Sync>;

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

/// Runs `chunk`.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the output is written, or the first error: a model or an
/// input that cannot be read, or an output that cannot be written.
fn chunk(args: ChunkArgs) -> Result<(), CommandError> {
    let bytes = read_model(&args.model)?;
    let model = Model::new(bytes).map_err(|error| format!("{}: {error}", args.model))?;
    let chunker = model
        .chunker()
        .ok_or_else(|| format!("{}: the model has no chunker", args.model))?;
    let mut sentences = read_lattice(&args.input)?;
    // Every sentence is chunked before the output is created: an input that
    // is not of the dictionary of the model leaves no file behind.
    for (index, sentence) in sentences.iter_mut().enumerate() {
        let ranges = chunker.chunk(&sentence.morphemes).map_err(|error| {
            format!("{}: sentence {}: {error}", args.input.display(), index + 1)
        })?;
        let count = ranges.len();
        // This command is the chunker alone: each bunsetsu depends on the
        // next, which is the baseline, and leaves the segmentation to be
        // evaluated. `parse --input-layer morphemes` segments and parses.
        sentence.bunsetsu = ranges
            .into_iter()
            .enumerate()
            .map(|(index, morphemes)| {
                Bunsetsu::new(morphemes, (index + 1 < count).then_some(index + 1))
            })
            .collect();
    }

    let output = File::create(&args.output)
        .map_err(|error| format!("cannot create {}: {error}", args.output.display()))?;
    let mut output = BufWriter::new(output);
    for sentence in &sentences {
        lattice::write(&mut output, sentence)
            .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;
    }
    output
        .flush()
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;
    Ok(())
}

/// Segments the morphemes of a sentence when a chunker is given, parses the
/// bunsetsu, and writes the sentence in the lattice format.
///
/// # Arguments
///
/// * `chunker` - The chunker, or `None` to parse the bunsetsu given.
/// * `parser` - The parser.
/// * `morphemes` - The morphemes of the sentence.
/// * `given` - The bunsetsu of the input, which are read without a chunker.
/// * `comments` - The comment lines of the sentence in the input.
/// * `output` - Where the sentence is written.
///
/// # Returns
///
/// `Ok(())`, or the error of the chunker, of the parser or of the writing.
fn write_parsed<D: ChunkerData, C: Classifier, M: Morpheme, W: Write>(
    chunker: Option<&Chunker<D>>,
    parser: &Parser<C>,
    morphemes: Vec<M>,
    given: &[Bunsetsu],
    comments: Vec<String>,
    output: &mut W,
) -> Result<(), CommandError> {
    let ranges = match chunker {
        Some(chunker) => chunker.chunk(&morphemes)?,
        None => given
            .iter()
            .map(|bunsetsu| bunsetsu.morphemes.clone())
            .collect(),
    };
    let bunsetsu = parser.parse(&morphemes, &ranges)?;
    let mut sentence = Sentence::new(morphemes, bunsetsu)?;
    sentence.comments = comments;
    lattice::write(output, &sentence)?;
    Ok(())
}

/// Runs `parse`.
///
/// A sentence is read, parsed and written before the next one is read, so
/// the command can stand in a pipe and its memory does not grow with the
/// input. When a sentence fails, what was written before it stays.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the output is written, or the first error: a model, a
/// dictionary or an input that cannot be read, a model without the part
/// that is needed or for another dictionary, a sentence that cannot be
/// parsed (the message names its line), or an output that cannot be
/// written.
fn parse(args: ParseArgs) -> Result<(), CommandError> {
    let bytes = read_model(&args.model)?;
    let model = Model::new(bytes).map_err(|error| format!("{}: {error}", args.model))?;
    let parser = model
        .parser()
        .ok_or_else(|| format!("{}: the model has no parser", args.model))?;
    let chunker = match args.input_layer {
        InputLayer::Bunsetsu => None,
        InputLayer::Text | InputLayer::Morphemes => Some(
            model
                .chunker()
                .ok_or_else(|| format!("{}: the model has no chunker", args.model))?,
        ),
    };

    let segmenter = match (args.input_layer, &args.dict) {
        (InputLayer::Text, None) => {
            return Err("--dict is needed to tokenize text: give the dictionary the model was trained for, or --input-layer morphemes for input that is tokenized".into());
        }
        (InputLayer::Text, Some(uri)) => {
            let dictionary = load_dictionary(uri)
                .map_err(|error| format!("cannot load the dictionary {uri}: {error}"))?;
            // A model gives wrong answers on the tokens of another
            // dictionary, without an error: the names are compared.
            model
                .check_dictionary(&dictionary.metadata.name)
                .map_err(|error| format!("{uri}: {error}"))?;
            // The default settings, as the training data was tokenized.
            Some(Segmenter::new(Mode::Normal, dictionary, None))
        }
        (_, Some(_)) => {
            return Err(
                "--dict is for the input layer text: a lattice input is tokenized already".into(),
            );
        }
        (_, None) => None,
    };

    let name = args.input.as_ref().map_or_else(
        || "the standard input".to_string(),
        |path| path.display().to_string(),
    );
    let input: Box<dyn BufRead> = match &args.input {
        Some(path) => Box::new(BufReader::new(
            File::open(path).map_err(|error| format!("cannot open {name}: {error}"))?,
        )),
        None => Box::new(io::stdin().lock()),
    };
    let mut output: Box<dyn Write> = match &args.output {
        Some(path) => {
            Box::new(BufWriter::new(File::create(path).map_err(|error| {
                format!("cannot create {}: {error}", path.display())
            })?))
        }
        None => Box::new(BufWriter::new(io::stdout().lock())),
    };

    match &segmenter {
        Some(segmenter) => {
            for (index, line) in input.lines().enumerate() {
                let named = |error: CommandError| format!("{name}: line {}: {error}", index + 1);
                let line = line.map_err(|error| named(error.into()))?;
                // One line is one sentence. `lines` leaves out the line
                // feed, and a carriage return before it.
                let mut tokens = segmenter
                    .segment(Cow::Borrowed(line.as_str()))
                    .map_err(|error| named(error.into()))?;
                let morphemes: Vec<DictionaryMorpheme> = tokens
                    .iter_mut()
                    .map(|token| {
                        let features = token.details_iter().map(str::to_string).collect();
                        DictionaryMorpheme::new(token.surface.to_string(), features)
                    })
                    .collect();
                write_parsed(
                    chunker.as_ref(),
                    &parser,
                    morphemes,
                    &[],
                    Vec::new(),
                    &mut output,
                )
                .map_err(named)?;
            }
        }
        None => {
            for (index, sentence) in lattice::Reader::new(input).enumerate() {
                let named =
                    |error: CommandError| format!("{name}: sentence {}: {error}", index + 1);
                let sentence = sentence.map_err(|error| format!("{name}: {error}"))?;
                write_parsed(
                    chunker.as_ref(),
                    &parser,
                    sentence.morphemes,
                    &sentence.bunsetsu,
                    sentence.comments,
                    &mut output,
                )
                .map_err(named)?;
            }
        }
    }
    output
        .flush()
        .map_err(|error| format!("cannot write the output: {error}"))?;
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    let result = match args.command {
        Command::Parse(args) => parse(args),
        Command::Chunk(args) => chunk(args),
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
