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
//!
//! `instances` writes the training instances of the parser: the decisions
//! the parser asks on each sentence of such training data, with their
//! features and their answers:
//!
//! ```text
//! lindera-dependency instances --input train.lattice --output train.instances
//! ```
//!
//! `train-parser` trains the parser on such training data and writes a
//! model file, with the chunker of another model file when one is given;
//! `parse` finds the heads of a file with a model, on the bunsetsu of the
//! file or, with `--chunk`, on those of the chunker of the model:
//!
//! ```text
//! lindera-dependency train-parser --input train.lattice --chunker chunker.model \\
//!     --output ipadic.model
//! lindera-dependency parse --model ipadic.model --input dev.lattice --output output.lattice
//! ```

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand};
use lindera_dependency::chunker::{self, Regularization, Template, TrainOptions};
use lindera_dependency::corpus::{self, DictionaryMorpheme, TokenizerError};
use lindera_dependency::evaluation::{self, Comparison, Count, Evaluation, Pairs};
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::{Model, ModelBuilder};
use lindera_dependency::parser::{
    self, Algorithm, FeatureSet, Pairs as PairTable, Quantization,
    TrainOptions as ParserTrainOptions,
};
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
    /// Writes the training instances of the parser.
    Instances(InstancesArgs),
    /// Trains the parser and writes a model file.
    TrainParser(TrainParserArgs),
    /// Finds the head of each bunsetsu of a file with a model file.
    Parse(ParseArgs),
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

/// The arguments of `instances`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Write the training instances of the parser: the decisions it asks on each sentence of a file in the lattice format, each with its features and with the answer of the file"
)]
struct InstancesArgs {
    /// The answers.
    #[clap(
        short = 'i',
        long = "input",
        help = "File with the answers, in the lattice format (the output of convert)"
    )]
    input: PathBuf,
    /// The instances.
    #[clap(
        short = 'o',
        long = "output",
        help = "File to write: one instance per line, +1 or -1 and the features separated by tabs, and an empty line after each sentence"
    )]
    output: PathBuf,
    /// The groups of features.
    #[clap(
        short = 'f',
        long = "features",
        default_value_t = FeatureSet::default().to_string(),
        help = "Groups of features to make, separated by commas"
    )]
    features: String,
}

/// The name of the dictionary of the models the command trains: the one
/// whose schema it uses, [`Schema::ipadic`].
const DICTIONARY: &str = "ipadic";

/// The algorithm of `train-parser`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum AlgorithmArg {
    /// Dual coordinate descent, as CaboCha.
    Dcd,
    /// Passive-aggressive, as J.DepP.
    Pa,
}

/// The table of the pairs of `train-parser`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum PairsArg {
    /// A weight of its own for every pair.
    Exact,
    /// A table of a fixed size the pairs are hashed into.
    Hashed,
}

/// The quantization of `train-parser`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum QuantizationArg {
    /// 4 bytes a weight.
    F32,
    /// 2 bytes a weight.
    I16,
    /// 1 byte a weight.
    I8,
}

/// The arguments of `train-parser`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Train the parser on a file in the lattice format with the head of every bunsetsu (the output of convert), for the IPADIC schema, and write a model file"
)]
struct TrainParserArgs {
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
    /// A model file to take the other parts from.
    #[clap(
        long = "chunker",
        help = "Model file that train-chunker wrote: its chunker goes into the output, which is then the whole model of the dictionary"
    )]
    chunker: Option<PathBuf>,
    /// The algorithm.
    #[clap(
        long = "algorithm",
        value_enum,
        default_value = "dcd",
        help = "dcd: dual coordinate descent on a support vector machine, as CaboCha; pa: passive-aggressive, as J.DepP"
    )]
    algorithm: AlgorithmArg,
    /// The cost.
    #[clap(
        long = "cost",
        help = "The most one training decision adds to the weights (default: 0.0015 for dcd, 0.001 for pa)"
    )]
    cost: Option<f64>,
    /// The passes of passive-aggressive.
    #[clap(
        long = "iterations",
        default_value_t = 10,
        help = "Passes over the training decisions, for pa"
    )]
    iterations: u32,
    /// Whether passive-aggressive averages.
    #[clap(
        long = "averaged",
        help = "For pa: the average of the weights after every decision, not the last ones"
    )]
    averaged: bool,
    /// The groups of features.
    #[clap(
        short = 'f',
        long = "features",
        default_value_t = FeatureSet::default().to_string(),
        help = "Groups of features to make, separated by commas"
    )]
    features: String,
    /// The table of the pairs.
    #[clap(
        long = "pairs",
        value_enum,
        default_value = "hashed",
        help = "exact: a weight for every pair; hashed: a table of 2^bits weights the pairs share"
    )]
    pairs: PairsArg,
    /// The support a pair needs.
    #[clap(
        long = "min-support",
        default_value_t = 1,
        help = "For exact pairs: the number of training decisions a pair has to be in to get a weight"
    )]
    min_support: u32,
    /// The size of the table of hashed pairs.
    #[clap(
        long = "bits",
        default_value_t = 22,
        help = "For hashed pairs: the table has 2^bits weights (1 to 30)"
    )]
    bits: u32,
    /// The count a feature needs.
    #[clap(
        long = "min-count",
        default_value_t = 1,
        help = "The number of training decisions a feature has to be in to be used"
    )]
    min_count: u32,
    /// The smallest weight of a pair that is kept.
    #[clap(
        long = "pair-threshold",
        default_value_t = 0.0,
        help = "For exact pairs: a pair whose weight is closer to 0 than this is left out of the model"
    )]
    pair_threshold: f64,
    /// How the weights are written.
    #[clap(
        long = "quantization",
        value_enum,
        default_value = "i8",
        help = "The weights in the file: f32 as they are, i16 or i8 as whole numbers times a scale"
    )]
    quantization: QuantizationArg,
    /// The seed.
    #[clap(
        long = "seed",
        default_value_t = 1,
        help = "Seed of the order of the decisions, for dcd"
    )]
    seed: u64,
}

/// The arguments of `parse`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Find the head of each bunsetsu of a file in the lattice format with a model file: on the bunsetsu of the input, or with --chunk on those of the chunker of the model"
)]
struct ParseArgs {
    /// The model.
    #[clap(
        short = 'm',
        long = "model",
        help = "Model file that train-parser wrote"
    )]
    model: PathBuf,
    /// The sentences.
    #[clap(
        short = 'i',
        long = "input",
        help = "File to parse, in the lattice format"
    )]
    input: PathBuf,
    /// The parsed sentences.
    #[clap(
        short = 'o',
        long = "output",
        help = "File to write, in the lattice format"
    )]
    output: PathBuf,
    /// Whether the morphemes are segmented again.
    #[clap(
        long = "chunk",
        help = "Segment the morphemes with the chunker of the model, and do not read the bunsetsu of the input: the analysis from morphemes to dependencies"
    )]
    chunk: bool,
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
    let bytes = ModelBuilder::new(DICTIONARY, options.schema.clone())?
        .chunker(&trained.model)?
        .to_bytes();
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
    let model = Model::new(bytes).map_err(|error| format!("{}: {error}", args.model.display()))?;
    let chunker = model
        .chunker()
        .ok_or_else(|| format!("{}: the model has no chunker", args.model.display()))?;
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
        // evaluated. `parse --chunk` segments and parses.
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

/// Runs `train-parser`.
///
/// The features are made with the schema of IPADIC.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the model and the report are written, or the first error:
/// a file that cannot be read or written, a feature set that cannot be
/// read, a model file whose dictionary is another one, or training data
/// that cannot be trained on.
fn train_parser(args: TrainParserArgs) -> Result<(), CommandError> {
    let options = ParserTrainOptions {
        features: FeatureSet::parse(&args.features)?,
        algorithm: match args.algorithm {
            AlgorithmArg::Dcd => Algorithm::DualCoordinateDescent {
                cost: args.cost.unwrap_or(0.0015),
            },
            AlgorithmArg::Pa => Algorithm::PassiveAggressive {
                cost: args.cost.unwrap_or(0.001),
                iterations: args.iterations,
                averaged: args.averaged,
            },
        },
        pairs: match args.pairs {
            PairsArg::Exact => PairTable::Exact {
                min_support: args.min_support,
            },
            PairsArg::Hashed => PairTable::Hashed { bits: args.bits },
        },
        min_count: args.min_count,
        pair_threshold: args.pair_threshold,
        seed: args.seed,
        ..ParserTrainOptions::default()
    };
    let quantization = match args.quantization {
        QuantizationArg::F32 => Quantization::F32,
        QuantizationArg::I16 => Quantization::I16,
        QuantizationArg::I8 => Quantization::I8,
    };
    // The file the chunker comes from is read before the training: a file
    // that cannot be used is found at once.
    let builder = match &args.chunker {
        None => ModelBuilder::new(DICTIONARY, options.schema.clone())?,
        Some(path) => {
            let bytes = fs::read(path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            let model =
                Model::new(bytes).map_err(|error| format!("{}: {error}", path.display()))?;
            model
                .check_dictionary(DICTIONARY)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if model.chunker().is_none() {
                return Err(format!("{}: the model has no chunker", path.display()).into());
            }
            ModelBuilder::from_model(&model)
        }
    };

    let sentences = read_lattice(&args.input)?;
    let started = Instant::now();
    let trained = parser::train(&sentences, &options)
        .map_err(|error| format!("{}: {error}", args.input.display()))?;
    let seconds = started.elapsed().as_secs_f64();
    let bytes = builder.parser(&trained.model, quantization)?.to_bytes();
    fs::write(&args.output, &bytes)
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;

    // A failure to write the summary is ignored: the model is complete.
    let _ = writeln!(
        io::stderr(),
        "{}: {} sentences\nfeatures: {}\ninstances: {}\npasses: {}{}\nupdates: {}\nfeature strings: {}\npair weights: {}\nbytes: {}\nseconds: {seconds:.1}",
        args.input.display(),
        sentences.len(),
        options.features,
        trained.instances,
        trained.passes,
        if trained.converged {
            ""
        } else {
            " (not converged)"
        },
        trained.updates,
        trained.model.feature_count(),
        trained.model.pair_count(),
        bytes.len()
    );
    Ok(())
}

/// Runs `parse`.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the output is written, or the first error: a model or an
/// input that cannot be read, a model without the part that is needed, a
/// sentence that cannot be parsed, or an output that cannot be written.
fn parse(args: ParseArgs) -> Result<(), CommandError> {
    let bytes = fs::read(&args.model)
        .map_err(|error| format!("cannot read {}: {error}", args.model.display()))?;
    let model = Model::new(bytes).map_err(|error| format!("{}: {error}", args.model.display()))?;
    let parser = model
        .parser()
        .ok_or_else(|| format!("{}: the model has no parser", args.model.display()))?;
    let chunker = if args.chunk {
        Some(
            model
                .chunker()
                .ok_or_else(|| format!("{}: the model has no chunker", args.model.display()))?,
        )
    } else {
        None
    };

    let mut sentences = read_lattice(&args.input)?;
    // Every sentence is parsed before the output is created: an input that
    // cannot be parsed leaves no file behind.
    for (index, sentence) in sentences.iter_mut().enumerate() {
        let named = |error: lindera_dependency::Error| {
            format!("{}: sentence {}: {error}", args.input.display(), index + 1)
        };
        let ranges = match &chunker {
            Some(chunker) => chunker.chunk(&sentence.morphemes).map_err(named)?,
            None => sentence
                .bunsetsu
                .iter()
                .map(|bunsetsu| bunsetsu.morphemes.clone())
                .collect(),
        };
        sentence.bunsetsu = parser.parse(&sentence.morphemes, &ranges).map_err(named)?;
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

/// Runs `instances`.
///
/// The features are made with the schema of IPADIC, as `convert` computes
/// the head words with it.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the output and the report are written, or the first
/// error: a feature set or an input that cannot be read, a sentence whose
/// tree the parser cannot make, or an output that cannot be written.
fn instances(args: InstancesArgs) -> Result<(), CommandError> {
    let set = FeatureSet::parse(&args.features)?;
    let schema = Schema::ipadic();
    let sentences = read_lattice(&args.input)?;

    let output = File::create(&args.output)
        .map_err(|error| format!("cannot create {}: {error}", args.output.display()))?;
    let mut output = BufWriter::new(output);
    let mut bunsetsu = 0;
    let mut positive = 0;
    let mut negative = 0;
    for (index, sentence) in sentences.iter().enumerate() {
        let instances = parser::instances(&schema, &set, sentence).map_err(|error| {
            format!("{}: sentence {}: {error}", args.input.display(), index + 1)
        })?;
        bunsetsu += sentence.bunsetsu.len();
        positive += instances.iter().filter(|one| one.label).count();
        negative += instances.iter().filter(|one| !one.label).count();
        parser::write_instances(&mut output, &instances)
            .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;
    }
    output
        .flush()
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;

    // A failure to write the report is ignored: the output is complete.
    let _ = writeln!(
        io::stderr(),
        "{}\nfeatures: {set}\nsentences: {}\nbunsetsu: {bunsetsu}\ninstances: {}\npositive: {positive}\nnegative: {negative}",
        args.input.display(),
        sentences.len(),
        positive + negative,
    );
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
        Command::Instances(args) => instances(args),
        Command::TrainParser(args) => train_parser(args),
        Command::Parse(args) => parse(args),
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
