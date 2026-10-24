//! Training functionality for custom morphological models.
//!
//! This module provides functions for training custom morphological analysis models
//! from annotated corpora.

use std::fs::File;
use std::path::Path;

use magnus::{Error, Integer, Ruby, function};

use lindera::dictionary::trainer::{
    Corpus, DEFAULT_LAMBDA, Model, SerializableModel, Trainer, TrainerConfig,
};

use crate::error::to_magnus_error;
use crate::util::optional_integer_arg;

/// Trains a morphological analysis model from an annotated corpus.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle (passed by magnus, not by the caller).
/// * `seed` - Seed lexicon file path (CSV format).
/// * `corpus` - Training corpus file path (annotated text).
/// * `char_def` - Character definition file path (char.def).
/// * `unk_def` - Unknown word definition file path (unk.def).
/// * `feature_def` - Feature definition file path (feature.def).
/// * `rewrite_def` - Rewrite rule definition file path (rewrite.def).
/// * `output` - Output model file path.
/// * `lambda` - L1 regularization (0.0-1.0), default: 0.01.
/// * `max_iter` - Maximum number of iterations, 0 or more, default: 100.
/// * `max_threads` - Maximum number of threads, 0 or more (nil = auto-detect
///   CPU cores).
///
/// # Returns
///
/// `Ok(())` on success, a `RangeError` for a `max_iter` or `max_threads`
/// outside its range (checked before the files), an `ArgumentError` for a
/// missing input file, or a `RuntimeError` when training fails.
#[allow(clippy::too_many_arguments)]
fn train(
    ruby: &Ruby,
    seed: String,
    corpus: String,
    char_def: String,
    unk_def: String,
    feature_def: String,
    rewrite_def: String,
    output: String,
    lambda: Option<f64>,
    max_iter: Option<Integer>,
    max_threads: Option<Integer>,
) -> Result<(), Error> {
    let max_iter: Option<u64> = optional_integer_arg(ruby, "max_iter", max_iter)?;
    let max_threads: Option<usize> = optional_integer_arg(ruby, "max_threads", max_threads)?;

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
    let lambda_val = lambda.unwrap_or(DEFAULT_LAMBDA);
    let max_iter_val = max_iter.unwrap_or(100);

    let trainer = Trainer::new(config)
        .map_err(|e| to_magnus_error(ruby, format!("Failed to initialize trainer: {e}")))?
        .regularization_cost(lambda_val)
        .max_iter(max_iter_val)
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
    module.define_module_function("train", function!(train, 10))?;
    module.define_module_function("export", function!(export, 3))?;
    Ok(())
}
