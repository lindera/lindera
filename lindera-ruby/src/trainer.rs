//! Training functionality for custom morphological models.
//!
//! This module provides functions for training custom morphological analysis models
//! from annotated corpora.

use std::fs::File;
use std::path::Path;

use magnus::scan_args::{get_kwargs, scan_args};
use magnus::{Error, Integer, RHash, Ruby, Value, function};

use lindera::dictionary::trainer::{
    Corpus, DEFAULT_ELASTIC_NET_L1_RATIO, DEFAULT_ETA, DEFAULT_LAMBDA, Model,
    ParseRegularizationTypeError, RegularizationType, SerializableModel, Trainer, TrainerConfig,
};
use lindera_binding::argument::{non_negative_float_argument, unit_interval_argument};

use crate::error::to_magnus_error;
use crate::util::optional_integer_arg;

/// Trains a morphological analysis model from an annotated corpus.
///
/// Ruby signature: `Lindera.train(seed, corpus, char_def, unk_def,
/// feature_def, rewrite_def, output, lambda = nil, max_iter = nil,
/// max_threads = nil, regularization: nil, elastic_net_l1_ratio: nil,
/// eta: nil)`.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle (passed by magnus, not by the caller).
/// * `args` - The Ruby arguments:
///   * `seed` - Seed lexicon file path (CSV format).
///   * `corpus` - Training corpus file path (annotated text).
///   * `char_def` - Character definition file path (char.def).
///   * `unk_def` - Unknown word definition file path (unk.def).
///   * `feature_def` - Feature definition file path (feature.def).
///   * `rewrite_def` - Rewrite rule definition file path (rewrite.def).
///   * `output` - Output model file path.
///   * `lambda` - Regularization coefficient, finite and >= 0, default
///     (also for `nil`): 1.0; with L2, MeCab's `-c C` is `1 / C`, so 1.0 is
///     MeCab's `-c 1.0`.
///   * `max_iter` - Maximum number of iterations, 0 or more, default: 100.
///   * `max_threads` - Maximum number of threads, 0 or more (nil =
///     auto-detect CPU cores).
///   * `regularization:` - Regularization type: `"l1"`, `"l2"` or
///     `"elasticnet"` (in any case; also `"elastic_net"` or
///     `"elastic-net"`), default (also for `nil`): `"l2"`.
///   * `elastic_net_l1_ratio:` - L1 ratio of Elastic Net regularization,
///     between 0 and 1, default (also for `nil`): 0.5; used only with
///     `"elasticnet"`.
///   * `eta:` - Tolerance of the convergence test, finite and >= 0, default
///     (also for `nil`): 5e-5: training stops before `max_iter` once the
///     relative change of the objective stays below it for 3 iterations in
///     a row; 0 disables the test.
///
/// # Returns
///
/// `Ok(())` on success; checked before the files, an `ArgumentError` for a
/// wrong number of arguments, an unknown keyword or an unknown
/// `regularization`, and a `RangeError` for a `lambda`, `max_iter`,
/// `max_threads`, `elastic_net_l1_ratio` or `eta` outside its range; then an
/// `ArgumentError` for a missing input file, or a `RuntimeError` when
/// training fails.
fn train(ruby: &Ruby, args: &[Value]) -> Result<(), Error> {
    let args = scan_args::<
        (String, String, String, String, String, String, String),
        (
            Option<Option<f64>>,
            Option<Option<Integer>>,
            Option<Option<Integer>>,
        ),
        (),
        (),
        RHash,
        (),
    >(args)?;
    let (seed, corpus, char_def, unk_def, feature_def, rewrite_def, output) = args.required;
    let (lambda, max_iter, max_threads) = args.optional;
    let keywords = get_kwargs::<
        _,
        (),
        (
            Option<Option<String>>,
            Option<Option<f64>>,
            Option<Option<f64>>,
        ),
        (),
    >(
        args.keywords,
        &[],
        &["regularization", "elastic_net_l1_ratio", "eta"],
    )?;
    let (regularization, elastic_net_l1_ratio, eta) = keywords.optional;

    let range_error = |message: String| Error::new(ruby.exception_range_error(), message);
    let regularization = match regularization.flatten() {
        Some(name) => name.parse().map_err(|e: ParseRegularizationTypeError| {
            Error::new(ruby.exception_arg_error(), e.to_string())
        })?,
        None => RegularizationType::default(),
    };
    let lambda_val =
        non_negative_float_argument("lambda", lambda.flatten().unwrap_or(DEFAULT_LAMBDA))
            .map_err(range_error)?;
    let elastic_net_l1_ratio = unit_interval_argument(
        "elastic_net_l1_ratio",
        elastic_net_l1_ratio
            .flatten()
            .unwrap_or(DEFAULT_ELASTIC_NET_L1_RATIO),
    )
    .map_err(range_error)?;
    let eta = non_negative_float_argument("eta", eta.flatten().unwrap_or(DEFAULT_ETA))
        .map_err(range_error)?;
    let max_iter: Option<u64> = optional_integer_arg(ruby, "max_iter", max_iter.flatten())?;
    let max_threads: Option<usize> =
        optional_integer_arg(ruby, "max_threads", max_threads.flatten())?;

    let seed_path = Path::new(&seed);
    let corpus_path = Path::new(&corpus);
    let char_def_path = Path::new(&char_def);
    let unk_def_path = Path::new(&unk_def);
    let feature_def_path = Path::new(&feature_def);
    let rewrite_def_path = Path::new(&rewrite_def);
    let output_path = Path::new(&output);

    // Validate input files
    for (path, name) in [
        (seed_path, "seed"),
        (corpus_path, "corpus"),
        (char_def_path, "char_def"),
        (unk_def_path, "unk_def"),
        (feature_def_path, "feature_def"),
        (rewrite_def_path, "rewrite_def"),
    ] {
        if !path.exists() {
            return Err(Error::new(
                ruby.exception_arg_error(),
                format!("{} file does not exist: {}", name, path.display()),
            ));
        }
    }

    // Load configuration
    let config = TrainerConfig::from_paths(
        seed_path,
        char_def_path,
        unk_def_path,
        feature_def_path,
        rewrite_def_path,
    )
    .map_err(|e| to_magnus_error(ruby, format!("Failed to load trainer configuration: {e}")))?;

    // Initialize trainer
    let num_threads = max_threads.unwrap_or_else(num_cpus::get);
    let max_iter_val = max_iter.unwrap_or(100);

    let trainer = Trainer::new(config)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to initialize trainer: {e}")))?
        .regularization_cost(lambda_val)
        .regularization_type(regularization, elastic_net_l1_ratio)
        .max_iter(max_iter_val)
        .eta(eta)
        .num_threads(num_threads);

    // Load corpus
    let corpus_file = File::open(corpus_path)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to open corpus file: {e}")))?;
    let corpus_data = Corpus::from_reader(corpus_file)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to load corpus: {e}")))?;

    println!("Training with {} examples...", corpus_data.len());

    // Train model
    let model = trainer
        .train(corpus_data)
        .map_err(|e| to_magnus_error(ruby, format!("Training failed: {e}")))?;

    // Save model
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            to_magnus_error(ruby, format!("Failed to create output directory: {e}"))
        })?;
    }

    let mut output_file = File::create(output_path)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to create output file: {e}")))?;

    model
        .write_model(&mut output_file)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to write model: {e}")))?;

    println!("Model saved to {}", output_path.display());
    Ok(())
}

/// Export dictionary files from trained model.
///
/// Writes the files `lindera export` writes, with the same method
/// (`SerializableModel::export`): `lex.csv`, `matrix.def`, `unk.def`,
/// `char.def`, `feature.def`, `rewrite.def`, `left-id.def` and
/// `right-id.def`, and `metadata.json` when `metadata` is given.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `model` - Trained model file path (.dat format).
/// * `output` - Output directory path.
/// * `metadata` - Optional base metadata.json file to update with trained model values.
///
/// # Returns
///
/// `Ok(())` on success, or a Magnus `Error` on failure.
fn export(
    ruby: &Ruby,
    model: String,
    output: String,
    metadata: Option<String>,
) -> Result<(), Error> {
    let model_path = Path::new(&model);
    let output_path = Path::new(&output);
    let metadata_path = metadata.as_deref().map(Path::new);

    if !model_path.exists() {
        return Err(Error::new(
            ruby.exception_arg_error(),
            format!("Model file does not exist: {}", model_path.display()),
        ));
    }
    if let Some(metadata_path) = metadata_path
        && !metadata_path.exists()
    {
        return Err(Error::new(
            ruby.exception_arg_error(),
            format!("Metadata file does not exist: {}", metadata_path.display()),
        ));
    }

    // Load trained model
    let model_file = File::open(model_path)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to open model file: {e}")))?;

    let serializable_model: SerializableModel = Model::read_model(model_file)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to load model: {e}")))?;

    // Export dictionary files
    let files_created = serializable_model
        .export(output_path, metadata_path)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to export dictionary files: {e}")))?;

    if metadata_path.is_some() {
        println!("Updated metadata.json with trained model values");
    }

    println!("Dictionary files exported to: {}", output_path.display());
    println!("Files created:");
    for file in &files_created {
        println!("  - {}", file.display());
    }

    Ok(())
}

/// Defines train and export module functions in the given Ruby module.
///
/// # Arguments
///
/// * `_ruby` - Ruby runtime handle.
/// * `module` - Parent Ruby module.
///
/// # Returns
///
/// `Ok(())` on success, or a Magnus `Error` on failure.
pub fn define(_ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    module.define_module_function("train", function!(train, -1))?;
    module.define_module_function("export", function!(export, 3))?;
    Ok(())
}
