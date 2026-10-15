//! Training functionality for custom morphological models.
//!
//! This module provides functions for training custom morphological analysis models
//! from annotated corpora. Requires the `train` feature.

use std::fs::File;
use std::path::Path;

use lindera::dictionary::trainer::{Corpus, Model, SerializableModel, Trainer, TrainerConfig};

use crate::error::to_napi_error;
use crate::util::optional_integer_arg;

/// Options for training a CRF model.
#[napi(object)]
pub struct TrainOptions {
    /// Path to the seed lexicon file (CSV format).
    pub seed: String,
    /// Path to the annotated training corpus.
    pub corpus: String,
    /// Path to the character definition file (char.def).
    pub char_def: String,
    /// Path to the unknown word definition file (unk.def).
    pub unk_def: String,
    /// Path to the feature definition file (feature.def).
    pub feature_def: String,
    /// Path to the rewrite rule definition file (rewrite.def).
    pub rewrite_def: String,
    /// Output path for the trained model file.
    pub output: String,
    /// L1 regularization cost (0.0-1.0, default: 0.01).
    pub lambda: Option<f64>,
    /// Maximum number of training iterations, an integer (default: 100).
    pub max_iter: Option<f64>,
    /// Number of threads, an integer (undefined = auto-detect CPU cores).
    pub max_threads: Option<f64>,
}

/// Trains a morphological analysis model from an annotated corpus.
///
/// # Arguments
///
/// * `options` - Training options containing file paths and parameters.
///
/// # Returns
///
/// Nothing, or an error. A `maxIter` or `maxThreads` that is not an integer
/// of 0 or more is an `InvalidArg` error, raised before any file is read.
#[napi]
pub fn train(options: TrainOptions) -> napi::Result<()> {
    // Check the settings before touching any file.
    let lambda = options.lambda.unwrap_or(0.01);
    let max_iter = optional_integer_arg("maxIter", options.max_iter)?.unwrap_or(100);
    let num_threads =
        optional_integer_arg("maxThreads", options.max_threads)?.unwrap_or_else(num_cpus::get);

    let seed_path = Path::new(&options.seed);
    let corpus_path = Path::new(&options.corpus);
    let char_def_path = Path::new(&options.char_def);
    let unk_def_path = Path::new(&options.unk_def);
    let feature_def_path = Path::new(&options.feature_def);
    let rewrite_def_path = Path::new(&options.rewrite_def);
    let output_path = Path::new(&options.output);

    // Validate input files
    for (path, name) in [
        (seed_path, "seed"),
        (corpus_path, "corpus"),
        (char_def_path, "charDef"),
        (unk_def_path, "unkDef"),
        (feature_def_path, "featureDef"),
        (rewrite_def_path, "rewriteDef"),
    ] {
        if !path.exists() {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
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
    .map_err(|e| to_napi_error(format!("Failed to load trainer configuration: {e}")))?;

    // Initialize trainer
    let trainer = Trainer::new(config)
        .map_err(|e| to_napi_error(format!("Failed to initialize trainer: {e}")))?
        .regularization_cost(lambda)
        .max_iter(max_iter)
        .num_threads(num_threads);

    // Load corpus
    let corpus_file = File::open(corpus_path)
        .map_err(|e| to_napi_error(format!("Failed to open corpus file: {e}")))?;
    let corpus = Corpus::from_reader(corpus_file)
        .map_err(|e| to_napi_error(format!("Failed to load corpus: {e}")))?;

    println!("Training with {} examples...", corpus.len());

    // Train model
    let model = trainer
        .train(corpus)
        .map_err(|e| to_napi_error(format!("Training failed: {e}")))?;

    // Save model
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| to_napi_error(format!("Failed to create output directory: {e}")))?;
    }

    let mut output_file = File::create(output_path)
        .map_err(|e| to_napi_error(format!("Failed to create output file: {e}")))?;

    model
        .write_model(&mut output_file)
        .map_err(|e| to_napi_error(format!("Failed to write model: {e}")))?;

    println!("Model saved to {}", output_path.display());
    Ok(())
}

/// Options for exporting a trained model.
#[napi(object)]
pub struct ExportOptions {
    /// Path to the trained model file (.dat).
    pub model: String,
    /// Output directory for dictionary source files.
    pub output: String,
    /// Optional path to a base metadata.json file.
    pub metadata: Option<String>,
}

/// Exports dictionary files from a trained model.
///
/// Writes the files `lindera export` writes, with the same method
/// (`SerializableModel::export`): `lex.csv`, `matrix.def`, `unk.def`,
/// `char.def`, `feature.def`, `rewrite.def`, `left-id.def` and
/// `right-id.def`, and `metadata.json` when `options.metadata` is given.
///
/// # Arguments
///
/// * `options` - Export options containing file paths.
#[napi]
pub fn export_model(options: ExportOptions) -> napi::Result<()> {
    let model_path = Path::new(&options.model);
    let output_path = Path::new(&options.output);
    let metadata_path = options.metadata.as_deref().map(Path::new);

    if !model_path.exists() {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("Model file does not exist: {}", model_path.display()),
        ));
    }
    if let Some(metadata_path) = metadata_path
        && !metadata_path.exists()
    {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("Metadata file does not exist: {}", metadata_path.display()),
        ));
    }

    // Load trained model
    let model_file = File::open(model_path)
        .map_err(|e| to_napi_error(format!("Failed to open model file: {e}")))?;

    let serializable_model: SerializableModel = Model::read_model(model_file)
        .map_err(|e| to_napi_error(format!("Failed to load model: {e}")))?;

    // Export dictionary files
    let files_created = serializable_model
        .export(output_path, metadata_path)
        .map_err(|e| to_napi_error(format!("Failed to export dictionary files: {e}")))?;

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
