//! The subcommands of `lindera dependency` that need the training code:
//! making training data, training the chunker and the parser, and
//! evaluating. They are in a build with the `train` feature.

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Instant;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_dependency::cabocha;
use lindera_dependency::chunker::{self, Regularization, Template, TrainOptions};
use lindera_dependency::corpus::{self, TokenizerError};
use lindera_dependency::evaluation::{self, Comparison, Count, Evaluation, Pairs};
use lindera_dependency::lattice;
use lindera_dependency::model::{Model, ModelBuilder};
use lindera_dependency::parser::{
    self, Algorithm, FeatureSet, Pairs as PairTable, Quantization,
    TrainOptions as ParserTrainOptions,
};
use lindera_dependency::{DictionaryMorpheme, Schema};

use super::{CommandError, read_lattice};
use crate::commands::write_stdout;

/// The arguments of `convert`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Make training data in the lattice format from a CoNLL-U file with bunsetsu labels (UD Japanese-GSD, UD Japanese-PUD)"
)]
pub(crate) struct ConvertArgs {
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
pub(crate) struct EvaluateArgs {
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
pub(crate) struct SplitArgs {
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
pub(crate) enum RegularizationArg {
    /// L1: a small model.
    L1,
    /// L2.
    L2,
}

/// The arguments of `train-chunker`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Train the model of the chunker on a file in the lattice format with bunsetsu (the output of convert)"
)]
pub(crate) struct TrainChunkerArgs {
    /// The dictionary the model is for.
    #[clap(
        long = "dictionary-name",
        default_value = DEFAULT_DICTIONARY,
        help = "Name of the dictionary the training data was tokenized with, whose schema is used: ipadic, unidic, unidic-cwj or unidic-csj"
    )]
    dictionary_name: String,
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
        help = "Coefficient of the regularization; the larger, the smaller the weights (default: 0.5 for ipadic, 0.25 for UniDic)"
    )]
    lambda: Option<f64>,
    /// The columns the template reads.
    #[clap(
        long = "columns",
        help = "Columns the template reads, by number: surface, pos, ctype, cform, base (default: surface,pos,cform for ipadic, surface,pos,cform,base for UniDic)"
    )]
    columns: Option<String>,
    /// The template.
    #[clap(
        long = "template",
        help = "Template file in the format of CRF++ (default: the template of CaboCha with the conjugation form as column 2, and for UniDic also the base form as column 3)"
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

/// The arguments of `instances`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Write the training instances of the parser: the decisions it asks on each sentence of a file in the lattice format, each with its features and with the answer of the file"
)]
pub(crate) struct InstancesArgs {
    /// The dictionary the model is for.
    #[clap(
        long = "dictionary-name",
        default_value = DEFAULT_DICTIONARY,
        help = "Name of the dictionary the training data was tokenized with, whose schema is used: ipadic, unidic, unidic-cwj or unidic-csj"
    )]
    dictionary_name: String,
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

/// The name of the dictionary the training commands take when none is
/// given: the one of the first model.
pub(crate) const DEFAULT_DICTIONARY: &str = "ipadic";

/// The schema of a dictionary, for a command.
///
/// # Arguments
///
/// * `dictionary` - The name of the dictionary.
///
/// # Returns
///
/// The schema, or an error that lists the dictionaries with a schema.
pub(crate) fn schema_of(dictionary: &str) -> Result<Schema, String> {
    Schema::for_dictionary(dictionary).ok_or_else(|| {
        format!(
            "there is no schema for the dictionary {dictionary}; there are schemas for {}",
            Schema::DICTIONARIES.join(", ")
        )
    })
}

/// The algorithm of `train-parser`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum AlgorithmArg {
    /// Dual coordinate descent, as CaboCha.
    Dcd,
    /// Passive-aggressive, as J.DepP.
    Pa,
}

/// The table of the pairs of `train-parser`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum PairsArg {
    /// A weight of its own for every pair.
    Exact,
    /// A table of a fixed size the pairs are hashed into.
    Hashed,
}

/// The quantization of `train-parser`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum QuantizationArg {
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
    about = "Train the parser on a file in the lattice format with the head of every bunsetsu (the output of convert), and write a model file"
)]
pub(crate) struct TrainParserArgs {
    /// The dictionary the model is for.
    #[clap(
        long = "dictionary-name",
        default_value = DEFAULT_DICTIONARY,
        help = "Name of the dictionary the training data was tokenized with, whose schema is used: ipadic, unidic, unidic-cwj or unidic-csj"
    )]
    dictionary_name: String,
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
pub(crate) fn convert(args: ConvertArgs) -> Result<(), CommandError> {
    let dictionary = load_dictionary(&args.dict)
        .map_err(|error| format!("cannot load the dictionary {}: {error}", args.dict))?;
    // The schema of the dictionary the data is tokenized with.
    let name = dictionary.metadata.name.clone();
    let schema = schema_of(&name).map_err(|error| format!("{}: {error}", args.dict))?;
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
    let report = corpus::convert(BufReader::new(input), &mut output, &schema, tokenize)
        .map_err(|error| format!("{}: {error}", args.input.display()))?;
    output
        .flush()
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;

    // A failure to write the report is ignored: the output is complete.
    let _ = writeln!(io::stderr(), "{}\n{report}", args.input.display());
    Ok(())
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
pub(crate) fn count_json(count: &Count) -> serde_json::Value {
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
pub(crate) fn evaluation_json(evaluation: &Evaluation) -> serde_json::Value {
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
pub(crate) fn pairs_json(pairs: &Pairs) -> serde_json::Value {
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
pub(crate) fn comparison_json(comparison: &Comparison) -> serde_json::Value {
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
pub(crate) fn evaluate(args: EvaluateArgs) -> Result<(), CommandError> {
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
    // A standard output without a reader ends the command quietly (#1316).
    write_stdout(&format!("{output}\n"))?;
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
pub(crate) fn split(args: SplitArgs) -> Result<(), CommandError> {
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
pub(crate) fn train_chunker(args: TrainChunkerArgs) -> Result<(), CommandError> {
    // The settings of the dictionary, for what no option gives.
    let schema = schema_of(&args.dictionary_name)?;
    let defaults = TrainOptions::for_dictionary(&args.dictionary_name).unwrap_or_default();
    let template = match &args.template {
        None => defaults.template,
        Some(path) => {
            let text = fs::read_to_string(path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            Template::parse(&text).map_err(|error| format!("{}: {error}", path.display()))?
        }
    };
    let options = TrainOptions {
        schema,
        columns: match &args.columns {
            None => defaults.columns,
            Some(columns) => chunker::parse_columns(columns)?,
        },
        template,
        regularization: match args.regularization {
            RegularizationArg::L1 => Regularization::L1,
            RegularizationArg::L2 => Regularization::L2,
        },
        lambda: args.lambda.unwrap_or(defaults.lambda),
        max_iterations: args.max_iterations,
        threads: args
            .threads
            .or_else(|| std::thread::available_parallelism().ok())
            .map_or(1, NonZeroUsize::get),
    };
    let sentences = read_lattice(&args.input)?;
    let trained = chunker::train(&sentences, &options)
        .map_err(|error| format!("{}: {error}", args.input.display()))?;
    let bytes = ModelBuilder::new(&args.dictionary_name, options.schema.clone())?
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

/// Runs `train-parser`.
///
/// The features are made with the schema of the dictionary of
/// `--dictionary-name`.
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
pub(crate) fn train_parser(args: TrainParserArgs) -> Result<(), CommandError> {
    let options = ParserTrainOptions {
        schema: schema_of(&args.dictionary_name)?,
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
    };
    let quantization = match args.quantization {
        QuantizationArg::F32 => Quantization::F32,
        QuantizationArg::I16 => Quantization::I16,
        QuantizationArg::I8 => Quantization::I8,
    };
    // The file the chunker comes from is read before the training: a file
    // that cannot be used is found at once.
    let builder = match &args.chunker {
        None => ModelBuilder::new(&args.dictionary_name, options.schema.clone())?,
        Some(path) => {
            let bytes = fs::read(path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            let model =
                Model::new(bytes).map_err(|error| format!("{}: {error}", path.display()))?;
            model
                .check_dictionary(&args.dictionary_name)
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

/// The arguments of `convert-cabocha`.
#[derive(Debug, clap::Args)]
#[clap(
    about = "Convert the chunker and parser models of CaboCha, in text, into a model file. The models of CaboCha are limited to research and personal use, and so is a converted model: do not distribute it"
)]
pub(crate) struct ConvertCabochaArgs {
    /// The chunker model of CaboCha.
    #[clap(
        long = "chunker",
        help = "Chunker model of CaboCha in text, as model/chunk.ipa.txt, model/chunk.juman.txt or model/chunk.unidic.txt of its source"
    )]
    chunker: PathBuf,
    /// The parser model of CaboCha.
    #[clap(
        long = "parser",
        help = "Parser model of CaboCha in text, for the same part-of-speech set as the chunker model: model/dep.ipa.txt, model/dep.juman.txt or model/dep.unidic.txt"
    )]
    parser: PathBuf,
    /// The model.
    #[clap(short = 'o', long = "output", help = "Model file to write")]
    output: PathBuf,
    /// The dictionary the model is for.
    #[clap(
        long = "dictionary",
        help = "Name of the dictionary the model is for, when it is not the first one with the part-of-speech set of the model: unidic-cwj or unidic-csj for the model for UniDic (default: ipadic, jumandic or unidic)"
    )]
    dictionary: Option<String>,
    /// The threshold of the weight of a pair.
    #[clap(
        long = "sigma",
        default_value_t = cabocha::Options::default().sigma,
        help = "A pair of features that is not of two frequent ones is kept when its weight is sigma times the share of the positive support vectors or more, or minus sigma times the share of the negative ones or less: -s of cabocha-model-index. 0 with --min-support 1 keeps every pair"
    )]
    sigma: f64,
    /// The number of support vectors a pair has to be in.
    #[clap(
        long = "min-support",
        default_value_t = cabocha::Options::default().min_support,
        help = "A pair of features that is not of two frequent ones is kept when it is in this many support vectors or more: -n of cabocha-model-index"
    )]
    min_support: usize,
    /// The number of frequent features.
    #[clap(
        long = "frequent-features",
        default_value_t = cabocha::Options::default().frequent_features,
        help = "The pairs of two of this many most frequent features are all kept: -F of cabocha-model-index"
    )]
    frequent_features: usize,
}

/// Runs `convert-cabocha`.
///
/// # Arguments
///
/// * `args` - The arguments.
///
/// # Returns
///
/// `Ok(())` once the model and the report are written, or the first error:
/// a file that cannot be read or written, files that are not the two text
/// models of CaboCha, or a model that is not supported.
pub(crate) fn convert_cabocha(args: ConvertCabochaArgs) -> Result<(), CommandError> {
    let read = |path: &PathBuf| {
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    };
    let chunker = read(&args.chunker)?;
    let parser = read(&args.parser)?;
    let options = cabocha::Options {
        sigma: args.sigma,
        min_support: args.min_support,
        frequent_features: args.frequent_features,
    };
    let started = Instant::now();
    let converted = cabocha::convert(&chunker, &parser, &options)?;
    let dictionary = args
        .dictionary
        .as_deref()
        .unwrap_or_else(|| converted.part_of_speech_set.dictionary());
    let bytes = converted.to_bytes_for(dictionary)?;
    let seconds = started.elapsed().as_secs_f64();
    fs::write(&args.output, &bytes)
        .map_err(|error| format!("cannot write {}: {error}", args.output.display()))?;

    // A failure to write the summary is ignored: the model is complete.
    let report = &converted.report;
    let _ = writeln!(
        io::stderr(),
        "dictionary: {}\nchunker feature strings: {}\nsupport vectors: {} ({} positive)\nparser feature strings: {}\npairs: {} of {} kept, {} with a weight\nunit: {:e}\nbytes: {}\nseconds: {seconds:.1}\nThe models of CaboCha are limited to research and personal use, and so is this file: do not distribute it.",
        dictionary,
        report.chunker_features,
        report.support_vectors,
        report.positives,
        report.parser_features,
        report.kept_pairs,
        report.all_pairs,
        report.pairs,
        report.unit,
        bytes.len()
    );
    Ok(())
}

/// Runs `instances`.
///
/// The features are made with the schema of the dictionary of
/// `--dictionary-name`, as `convert` computes the head words with it.
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
pub(crate) fn instances(args: InstancesArgs) -> Result<(), CommandError> {
    let set = FeatureSet::parse(&args.features)?;
    let schema = schema_of(&args.dictionary_name)?;
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
