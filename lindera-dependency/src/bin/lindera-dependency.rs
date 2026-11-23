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
//! goes to the standard error. The report ends with how often the head word
//! of the IPADIC schema is the content head of the corpus.
//!
//! `evaluate` compares an output with the answer, both in the lattice
//! format, and writes the accuracy of chunking and of dependency to the
//! standard output; with a second output, also the paired counts of the two
//! and the p-value of McNemar's test:
//!
//! ```text
//! lindera-dependency evaluate --gold dev.lattice --system output.lattice
//! ```
//!
//! `split` writes the test and the training file of each fold of a
//! cross-validation:
//!
//! ```text
//! lindera-dependency split --input train.lattice --folds 10 --output-dir folds
//! ```
//!
//! `train-chunker` trains the model of the chunker on such training data,
//! and `chunk` segments the morphemes of a file again with a model, for
//! `evaluate` to compare with the answer:
//!
//! ```text
//! lindera-dependency train-chunker --input train.lattice --output chunker.model
//! lindera-dependency chunk --model chunker.model --input dev.lattice --output output.lattice
//! ```

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use lindera_dependency::chunker::{
    self, Chunker, ChunkerModel, Regularization, Template, TrainOptions,
};
use lindera_dependency::corpus::{self, DictionaryMorpheme, TokenizerError};
use lindera_dependency::evaluation::{self, Comparison, Count, Evaluation, Pairs};
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::{Bunsetsu, Schema, Sentence};
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
    /// Evaluates an output against the answer.
    Evaluate(EvaluateArgs),
    /// Splits a file into the folds of a cross-validation.
    Split(SplitArgs),
    /// Trains the model of the chunker.
    TrainChunker(TrainChunkerArgs),
    /// Segments the morphemes of a file into bunsetsu with a model.
    Chunk(ChunkArgs),
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

/// The arguments of `evaluate`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Evaluate chunking and dependency accuracy of an output against the answer, both in the lattice format, as CaboCha and J.DepP count them"
)]
struct EvaluateArgs {
    /// The answer.
    #[clap(short = 'g', long = "gold", help = "The answer, in the lattice format")]
    gold: PathBuf,
    /// The output to evaluate.
    #[clap(
        short = 's',
        long = "system",
        help = "The output to evaluate: the same sentences in the same order"
    )]
    system: PathBuf,
    /// A second output, to compare with the first.
    #[clap(
        short = 'c',
        long = "compare",
        help = "A second output: also evaluated, and paired with the first for McNemar's test"
    )]
    compare: Option<PathBuf>,
    /// Whether to write JSON.
    #[clap(long = "json", help = "Write JSON instead of text")]
    json: bool,
}

/// The arguments of `split`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Split a file in the lattice format into the folds of a cross-validation: sentence i is in the test file of fold i mod k, and in the training file of every other fold"
)]
struct SplitArgs {
    /// The file to split.
    #[clap(
        short = 'i',
        long = "input",
        help = "File to split, in the lattice format"
    )]
    input: PathBuf,
    /// The number of folds.
    #[clap(short = 'k', long = "folds", help = "Number of folds, at least 2")]
    folds: NonZeroUsize,
    /// Where the files of the folds are written.
    #[clap(
        short = 'o',
        long = "output-dir",
        help = "Directory for fold<i>.test.lattice and fold<i>.train.lattice, created when missing"
    )]
    output_dir: PathBuf,
}

/// The regularization of `train-chunker`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum RegularizationArg {
    /// L1: a small model.
    L1,
    /// L2.
    L2,
}

/// The arguments of `train-chunker`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Train the model of the chunker on a file in the lattice format with bunsetsu (the output of convert), for the IPADIC schema"
)]
struct TrainChunkerArgs {
    /// The training data.
    #[clap(
        short = 'i',
        long = "input",
        help = "Training data, in the lattice format"
    )]
    input: PathBuf,
    /// The model.
    #[clap(short = 'o', long = "output", help = "Model file to write")]
    output: PathBuf,
    /// The regularization.
    #[clap(
        long = "regularization",
        value_enum,
        default_value = "l1",
        help = "Regularization: l1 gives a small model, and the better one on UD Japanese-GSD"
    )]
    regularization: RegularizationArg,
    /// The coefficient of the regularization.
    #[clap(
        long = "lambda",
        default_value_t = 0.5,
        help = "Coefficient of the regularization; the larger, the smaller the weights"
    )]
    lambda: f64,
    /// The columns the template reads.
    #[clap(
        long = "columns",
        default_value = "surface,pos,cform",
        help = "Columns the template reads, by number: surface, pos, ctype, cform, base"
    )]
    columns: String,
    /// The template.
    #[clap(
        long = "template",
        help = "Template file in the format of CRF++ (default: the template of CaboCha, and the conjugation form as column 2)"
    )]
    template: Option<PathBuf>,
    /// The largest number of iterations.
    #[clap(
        long = "max-iterations",
        default_value_t = 1000,
        help = "Largest number of iterations of the optimizer"
    )]
    max_iterations: u64,
    /// The number of threads.
    #[clap(
        long = "threads",
        help = "Number of threads (default: the number of CPUs)"
    )]
    threads: Option<NonZeroUsize>,
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
        help = "Model file that train-chunker wrote"
    )]
    model: PathBuf,
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
    let report = corpus::convert(
        BufReader::new(input),
        &mut output,
        &Schema::ipadic(),
        tokenize,
    )
    .map_err(|error| format!("{}: {error}", args.input.display()))?;
    output
        .flush()
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;

    // A failure to write the report is ignored: the output is complete.
    let _ = writeln!(io::stderr(), "{}\n{report}", args.input.display());
    Ok(())
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

/// A count as JSON: its numerator, its denominator and their ratio, `null`
/// when nothing was counted.
///
/// # Arguments
///
/// * `count` - The count.
///
/// # Returns
///
/// The JSON object.
fn count_json(count: &Count) -> serde_json::Value {
    serde_json::json!({
        "correct": count.correct,
        "total": count.total,
        "ratio": count.ratio(),
    })
}

/// An evaluation as JSON.
///
/// # Arguments
///
/// * `evaluation` - The evaluation.
///
/// # Returns
///
/// The JSON object.
fn evaluation_json(evaluation: &Evaluation) -> serde_json::Value {
    let chunking = &evaluation.chunking;
    serde_json::json!({
        "sentences": evaluation.sentences,
        "different_morphemes": evaluation.different,
        "without_bunsetsu": evaluation.without_bunsetsu,
        "chunking": {
            "correct": chunking.correct,
            "system": chunking.system,
            "gold": chunking.gold,
            "precision": chunking.precision(),
            "recall": chunking.recall(),
            "f_score": chunking.f_score(),
            "sentences": count_json(&chunking.sentences),
        },
        "dependency": {
            "level0": count_json(&evaluation.dependency.level0),
            "level1": count_json(&evaluation.dependency.level1),
            "level2": count_json(&evaluation.dependency.level2),
            "sentences": count_json(&evaluation.dependency.sentences),
        },
        "end_to_end": {
            "arcs": count_json(&evaluation.end_to_end.arcs),
            "sentences": count_json(&evaluation.end_to_end.sentences),
        },
    })
}

/// The pairs of two outputs as JSON, with the p-value of McNemar's test.
///
/// # Arguments
///
/// * `pairs` - The pairs.
///
/// # Returns
///
/// The JSON object.
fn pairs_json(pairs: &Pairs) -> serde_json::Value {
    serde_json::json!({
        "both": pairs.both,
        "first_only": pairs.first_only,
        "second_only": pairs.second_only,
        "neither": pairs.neither,
        "p_value": pairs.p_value(),
    })
}

/// A comparison as JSON.
///
/// # Arguments
///
/// * `comparison` - The comparison.
///
/// # Returns
///
/// The JSON object.
fn comparison_json(comparison: &Comparison) -> serde_json::Value {
    serde_json::json!({
        "first": evaluation_json(&comparison.first),
        "second": evaluation_json(&comparison.second),
        "paired_segmentations": pairs_json(&comparison.segmentations),
        "paired_arcs": pairs_json(&comparison.arcs),
        "paired_sentences": pairs_json(&comparison.sentences),
    })
}

/// Runs `evaluate`.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the result is written to the standard output, or the first
/// error: a file that cannot be read, files that hold different numbers of
/// sentences, or a failure to write.
fn evaluate(args: EvaluateArgs) -> Result<(), CommandError> {
    let gold = read_lattice(&args.gold)?;
    let system = read_lattice(&args.system)?;
    let (text, json) = match &args.compare {
        None => {
            let evaluation = evaluation::evaluate(&gold, &system)?;
            (evaluation.to_string(), evaluation_json(&evaluation))
        }
        Some(second) => {
            let second = read_lattice(second)?;
            let comparison = evaluation::compare(&gold, &system, &second)?;
            (comparison.to_string(), comparison_json(&comparison))
        }
    };
    let output = if args.json {
        serde_json::to_string_pretty(&json)?
    } else {
        text
    };
    writeln!(io::stdout(), "{output}")?;
    Ok(())
}

/// Runs `split`.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the files of every fold are written, or the first error:
/// fewer than 2 folds, a file that cannot be read, or a file that cannot be
/// written.
fn split(args: SplitArgs) -> Result<(), CommandError> {
    let folds = args.folds.get();
    if folds < 2 {
        return Err("a cross-validation needs at least 2 folds".into());
    }
    let sentences = read_lattice(&args.input)?;
    fs::create_dir_all(&args.output_dir)
        .map_err(|error| format!("cannot create {}: {error}", args.output_dir.display()))?;

    let create = |name: String| -> Result<(PathBuf, BufWriter<File>), CommandError> {
        let path = args.output_dir.join(name);
        let file = File::create(&path)
            .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
        Ok((path, BufWriter::new(file)))
    };
    let mut tests = Vec::with_capacity(folds);
    let mut trains = Vec::with_capacity(folds);
    for fold in 0..folds {
        tests.push(create(format!("fold{fold}.test.lattice"))?);
        trains.push(create(format!("fold{fold}.train.lattice"))?);
    }

    let mut test_sizes = vec![0usize; folds];
    for (index, sentence) in sentences.iter().enumerate() {
        let own = evaluation::fold_of(index, args.folds);
        test_sizes[own] += 1;
        for fold in 0..folds {
            let (path, writer) = if fold == own {
                &mut tests[fold]
            } else {
                &mut trains[fold]
            };
            lattice::write(writer, sentence)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        }
    }
    for (path, writer) in tests.iter_mut().chain(trains.iter_mut()) {
        writer
            .flush()
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }

    // A failure to write the summary is ignored: the files are complete.
    let mut summary = format!("{}: {} sentences", args.input.display(), sentences.len());
    for (fold, size) in test_sizes.iter().enumerate() {
        summary.push_str(&format!(
            "\nfold {fold}: test {size}, train {}",
            sentences.len() - size
        ));
    }
    let _ = writeln!(io::stderr(), "{summary}");
    Ok(())
}

/// Runs `train-chunker`.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the model is written, or the first error: a file that
/// cannot be read or written, columns or a template that cannot be read, or
/// training data that cannot be trained on.
fn train_chunker(args: TrainChunkerArgs) -> Result<(), CommandError> {
    let template = match &args.template {
        None => Template::default(),
        Some(path) => {
            let text = fs::read_to_string(path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            Template::parse(&text).map_err(|error| format!("{}: {error}", path.display()))?
        }
    };
    let options = TrainOptions {
        columns: chunker::parse_columns(&args.columns)?,
        template,
        regularization: match args.regularization {
            RegularizationArg::L1 => Regularization::L1,
            RegularizationArg::L2 => Regularization::L2,
        },
        lambda: args.lambda,
        max_iterations: args.max_iterations,
        threads: args
            .threads
            .or_else(|| std::thread::available_parallelism().ok())
            .map_or(1, NonZeroUsize::get),
        ..TrainOptions::default()
    };
    let sentences = read_lattice(&args.input)?;
    let trained = chunker::train(&sentences, &options)
        .map_err(|error| format!("{}: {error}", args.input.display()))?;
    let bytes = trained.model.to_bytes();
    fs::write(&args.output, &bytes)
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;

    // A failure to write the summary is ignored: the model is complete.
    let _ = writeln!(
        io::stderr(),
        "{}: {} sentences\niterations: {}{}\nfeature strings: {}\nweights: {}\nbytes: {}",
        args.input.display(),
        sentences.len(),
        trained.iterations,
        if trained.converged {
            ""
        } else {
            " (not converged)"
        },
        trained.model.feature_count(),
        trained.model.weight_count(),
        bytes.len()
    );
    Ok(())
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
    let bytes = fs::read(&args.model)
        .map_err(|error| format!("cannot read {}: {error}", args.model.display()))?;
    let model = ChunkerModel::from_bytes(&bytes)
        .map_err(|error| format!("{}: {error}", args.model.display()))?;
    let chunker = Chunker::new(model);
    let sentences = read_lattice(&args.input)?;

    let output = File::create(&args.output)
        .map_err(|error| format!("cannot create {}: {error}", args.output.display()))?;
    let mut output = BufWriter::new(output);
    for mut sentence in sentences {
        let ranges = chunker.chunk(&sentence.morphemes);
        let count = ranges.len();
        // There is no parser yet: each bunsetsu depends on the next, which
        // is the baseline, and leaves the segmentation to be evaluated.
        sentence.bunsetsu = ranges
            .into_iter()
            .enumerate()
            .map(|(index, morphemes)| {
                Bunsetsu::new(morphemes, (index + 1 < count).then_some(index + 1))
            })
            .collect();
        lattice::write(&mut output, &sentence)
            .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;
    }
    output
        .flush()
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    let result = match args.command {
        Command::Convert(args) => convert(args),
        Command::Evaluate(args) => evaluate(args),
        Command::Split(args) => split(args),
        Command::TrainChunker(args) => train_chunker(args),
        Command::Chunk(args) => chunk(args),
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
