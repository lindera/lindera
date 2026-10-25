//! Training functionality for custom morphological models.
//!
//! This module provides functions for training custom morphological analysis models
//! from annotated corpora.
//!
//! # Examples
//!
//! ```python
//! # Train a model
//! lindera.train(
//!     seed="seed.csv",
//!     corpus="corpus.txt",
//!     char_def="char.def",
//!     unk_def="unk.def",
//!     feature_def="feature.def",
//!     rewrite_def="rewrite.def",
//!     output="model.bin",
//!     lambda_=1.0,
//!     max_iter=100,
//!     regularization="l2",
//! )
//!
//! # Export trained model
//! lindera.export(
//!     model_file="model.bin",
//!     output_dir="/path/to/output"
//! )
//! ```

use std::fs::File;
use std::path::Path;

use pyo3::{exceptions::PyValueError, prelude::*};

use lindera::dictionary::trainer::{
    Corpus, DEFAULT_ELASTIC_NET_L1_RATIO, DEFAULT_ETA, DEFAULT_LAMBDA, Model,
    ParseRegularizationTypeError, RegularizationType, SerializableModel, Trainer, TrainerConfig,
};
use lindera_binding::argument::{
    Positive, integer_argument_message, non_negative_float_argument, unit_interval_argument,
};

// The defaults of `lambda_`, `elastic_net_l1_ratio` and `eta` in the
// signature below are literals, which `help()` shows; they must be the
// core's.
const _: () = assert!(
    DEFAULT_LAMBDA == 1.0,
    "update the default of lambda_ in the signature of train"
);
const _: () = assert!(
    DEFAULT_ELASTIC_NET_L1_RATIO == 0.5,
    "update the default of elastic_net_l1_ratio in the signature of train"
);
const _: () = assert!(
    DEFAULT_ETA == 5e-5,
    "update the default of eta in the signature of train"
);

/// Trains a morphological analysis model from an annotated corpus.
///
/// # Arguments
///
/// * `seed` - Seed lexicon file path (CSV format)
/// * `corpus` - Training corpus file path (annotated text)
/// * `char_def` - Character definition file path (char.def)
/// * `unk_def` - Unknown word definition file path (unk.def)
/// * `feature_def` - Feature definition file path (feature.def)
/// * `rewrite_def` - Rewrite rule definition file path (rewrite.def)
/// * `output` - Output model file path
/// * `lambda_` - Regularization coefficient, finite and >= 0, default: 1.0;
///   with L2, MeCab's `-c C` is `1 / C`, so 1.0 is MeCab's `-c 1.0`.
/// * `max_iter` - Maximum number of iterations, 1 or more, default: 100
/// * `max_threads` - Maximum number of threads, 1 or more (None =
///   auto-detect CPU cores)
/// * `regularization` - Regularization type: `"l1"`, `"l2"` or
///   `"elasticnet"` (in any case; also `"elastic_net"` or `"elastic-net"`),
///   default: `"l2"`.
/// * `elastic_net_l1_ratio` - L1 ratio of Elastic Net regularization,
///   between 0 and 1, default: 0.5; used only with `"elasticnet"`.
/// * `eta` - Tolerance of the convergence test, finite and >= 0, default:
///   5e-5: training stops before `max_iter` once the relative change of the
///   objective stays below it for 3 iterations in a row; 0 disables the
///   test.
///
/// # Returns
///
/// * `PyResult<()>` - Returns Ok(()) on success, or a `ValueError`, raised
///   before any file is read for a `max_iter` or `max_threads` of 0 and a
///   `regularization`, `lambda_`, `elastic_net_l1_ratio` or `eta` that is
///   not accepted. A negative `max_iter` or `max_threads` is pyo3's
///   `OverflowError`.
#[pyfunction]
#[pyo3(signature = (seed, corpus, char_def, unk_def, feature_def, rewrite_def, output, lambda_=1.0, max_iter=100, max_threads=None, regularization="l2", elastic_net_l1_ratio=0.5, eta=5e-5))]
#[allow(clippy::too_many_arguments)]
pub fn train(
    seed: &str,
    corpus: &str,
    char_def: &str,
    unk_def: &str,
    feature_def: &str,
    rewrite_def: &str,
    output: &str,
    lambda_: f64,
    max_iter: u64,
    max_threads: Option<usize>,
    regularization: &str,
    elastic_net_l1_ratio: f64,
    eta: f64,
) -> PyResult<()> {
    // Check the settings before touching any file.
    let regularization: RegularizationType = regularization
        .parse()
        .map_err(|e: ParseRegularizationTypeError| PyValueError::new_err(e.to_string()))?;
    let lambda_ = non_negative_float_argument("lambda_", lambda_).map_err(PyValueError::new_err)?;
    let elastic_net_l1_ratio = unit_interval_argument("elastic_net_l1_ratio", elastic_net_l1_ratio)
        .map_err(PyValueError::new_err)?;
    let eta = non_negative_float_argument("eta", eta).map_err(PyValueError::new_err)?;
    // pyo3 rejects a negative count; the trainer would reject 0 only after
    // reading the corpus and building every lattice.
    if max_iter == 0 {
        return Err(PyValueError::new_err(integer_argument_message::<
            Positive<u64>,
        >("max_iter", 0)));
    }
    if max_threads == Some(0) {
        return Err(PyValueError::new_err(integer_argument_message::<
            Positive<usize>,
        >("max_threads", 0)));
    }

    let seed_path = Path::new(seed);
    let corpus_path = Path::new(corpus);
    let char_def_path = Path::new(char_def);
    let unk_def_path = Path::new(unk_def);
    let feature_def_path = Path::new(feature_def);
    let rewrite_def_path = Path::new(rewrite_def);
    let output_path = Path::new(output);

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
            return Err(PyValueError::new_err(format!(
                "{} file does not exist: {}",
                name,
                path.display()
            )));
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
    .map_err(|e| PyValueError::new_err(format!("Failed to load trainer configuration: {e}")))?;

    // Initialize trainer
    let num_threads = max_threads.unwrap_or_else(num_cpus::get);
    let trainer = Trainer::new(config)
        .map_err(|e| PyValueError::new_err(format!("Failed to initialize trainer: {e}")))?
        .regularization_cost(lambda_)
        .regularization_type(regularization, elastic_net_l1_ratio)
        .max_iter(max_iter)
        .eta(eta)
        .num_threads(num_threads);

    // Load corpus
    let corpus_file = File::open(corpus_path)
        .map_err(|e| PyValueError::new_err(format!("Failed to open corpus file: {e}")))?;
    let corpus = Corpus::from_reader(corpus_file)
        .map_err(|e| PyValueError::new_err(format!("Failed to load corpus: {e}")))?;

    println!("Training with {} examples...", corpus.len());

    // Train model
    let model = trainer
        .train(corpus)
        .map_err(|e| PyValueError::new_err(format!("Training failed: {e}")))?;

    // Save model
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            PyValueError::new_err(format!("Failed to create output directory: {e}"))
        })?;
    }

    let mut output_file = File::create(output_path)
        .map_err(|e| PyValueError::new_err(format!("Failed to create output file: {e}")))?;

    model
        .write_model(&mut output_file)
        .map_err(|e| PyValueError::new_err(format!("Failed to write model: {e}")))?;

    println!("Model saved to {}", output_path.display());
    Ok(())
}

/// Export dictionary files from trained model
///
/// Writes the files `lindera export` writes, with the same method
/// (`SerializableModel::export`): `lex.csv`, `matrix.def`, `unk.def`,
/// `char.def`, `feature.def`, `rewrite.def`, `left-id.def` and
/// `right-id.def`, and `metadata.json` when `metadata` is given.
///
/// # Arguments
///
/// * `model` - Trained model file path (.dat format)
/// * `output` - Output directory path
/// * `metadata` - Optional base metadata.json file to update with trained model values
///
/// # Returns
///
/// * `PyResult<()>` - Returns Ok(()) on success
#[pyfunction]
#[pyo3(signature = (model, output, metadata=None))]
pub fn export(model: &str, output: &str, metadata: Option<&str>) -> PyResult<()> {
    let model_path = Path::new(model);
    let output_path = Path::new(output);
    let metadata_path = metadata.map(Path::new);

    // Validate input files
    if !model_path.exists() {
        return Err(PyValueError::new_err(format!(
            "Model file does not exist: {}",
            model_path.display()
        )));
    }
    if let Some(metadata_path) = metadata_path
        && !metadata_path.exists()
    {
        return Err(PyValueError::new_err(format!(
            "Metadata file does not exist: {}",
            metadata_path.display()
        )));
    }

    // Load trained model
    let model_file = File::open(model_path)
        .map_err(|e| PyValueError::new_err(format!("Failed to open model file: {e}")))?;

    let serializable_model: SerializableModel = Model::read_model(model_file)
        .map_err(|e| PyValueError::new_err(format!("Failed to load model: {e}")))?;

    // Export dictionary files
    let files_created = serializable_model
        .export(output_path, metadata_path)
        .map_err(|e| PyValueError::new_err(format!("Failed to export dictionary files: {e}")))?;

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
