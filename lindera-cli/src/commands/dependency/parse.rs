//! `lindera dependency parse` and `lindera dependency chunk`: the two
//! subcommands for a user, in a build without training code.

use std::borrow::Cow;
use std::cell::Cell;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_dependency::chunker::{Chunker, ChunkerData};
use lindera_dependency::lattice;
use lindera_dependency::model::Model;
use lindera_dependency::parser::{Classifier, Parser};
use lindera_dependency::{Bunsetsu, DictionaryMorpheme, Morpheme, Sentence};

use super::{CommandError, read_lattice, read_model};
use crate::commands::stdout_err;

/// What the input of `parse` holds already.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(super) enum InputLayer {
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
pub(super) struct ParseArgs {
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
pub(super) struct ChunkArgs {
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

/// The standard output, which remembers that its reader has exited.
///
/// A write that fails goes through the messages of the subcommand, which
/// name the sentence; the flag is what tells `parse` that the failure was a
/// standard output without a reader.
struct StandardOutput<W> {
    /// The standard output.
    inner: W,
    /// Set once a write or a flush failed with a broken pipe.
    closed: Rc<Cell<bool>>,
}

impl<W> StandardOutput<W> {
    /// Notes a broken pipe, and gives the result back.
    ///
    /// # Arguments
    ///
    /// * `result` - The result of a write or of a flush.
    ///
    /// # Returns
    ///
    /// `result`, unchanged.
    fn note<T>(&self, result: io::Result<T>) -> io::Result<T> {
        if matches!(&result, Err(error) if error.kind() == io::ErrorKind::BrokenPipe) {
            self.closed.set(true);
        }
        result
    }
}

impl<W: Write> Write for StandardOutput<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let result = self.inner.write(buffer);
        self.note(result)
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = self.inner.flush();
        self.note(result)
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
pub(super) fn chunk(args: ChunkArgs) -> Result<(), CommandError> {
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
/// When the reader of the standard output has exited, as `head` does once
/// it has its lines, the command stops and returns the error `main` ends
/// quietly on, as the other commands of `lindera` do (#1316).
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
pub(super) fn parse(args: ParseArgs) -> Result<(), CommandError> {
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
    // Set by a write to the standard output that found no reader.
    let closed = Rc::new(Cell::new(false));
    let mut output: Box<dyn Write> = match &args.output {
        Some(path) => {
            Box::new(BufWriter::new(File::create(path).map_err(|error| {
                format!("cannot create {}: {error}", path.display())
            })?))
        }
        None => Box::new(BufWriter::new(StandardOutput {
            inner: io::stdout().lock(),
            closed: Rc::clone(&closed),
        })),
    };

    let result = write_sentences(
        segmenter.as_ref(),
        chunker.as_ref(),
        &parser,
        &name,
        input,
        &mut *output,
    );
    if closed.get() {
        // Whatever message the failed write got on its way, the cause is
        // that nobody reads the output.
        return Err(Box::new(stdout_err(io::Error::from(
            io::ErrorKind::BrokenPipe,
        ))));
    }
    result
}

/// Reads the sentences of the input, parses each and writes it, one at a
/// time.
///
/// # Arguments
///
/// * `segmenter` - The segmenter that tokenizes the lines of a text input,
///   or `None` for an input in the lattice format.
/// * `chunker` - The chunker, or `None` to parse the bunsetsu of the input.
/// * `parser` - The parser.
/// * `name` - The name of the input, for the messages.
/// * `input` - The input.
/// * `output` - Where the sentences are written.
///
/// # Returns
///
/// `Ok(())` once the output is written and flushed, or the first error,
/// which names the line or the sentence.
fn write_sentences<D: ChunkerData, C: Classifier>(
    segmenter: Option<&Segmenter>,
    chunker: Option<&Chunker<D>>,
    parser: &Parser<C>,
    name: &str,
    input: Box<dyn BufRead>,
    mut output: &mut dyn Write,
) -> Result<(), CommandError> {
    match segmenter {
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
                write_parsed(chunker, parser, morphemes, &[], Vec::new(), &mut output)
                    .map_err(named)?;
            }
        }
        None => {
            for (index, sentence) in lattice::Reader::new(input).enumerate() {
                let named =
                    |error: CommandError| format!("{name}: sentence {}: {error}", index + 1);
                let sentence = sentence.map_err(|error| format!("{name}: {error}"))?;
                write_parsed(
                    chunker,
                    parser,
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
