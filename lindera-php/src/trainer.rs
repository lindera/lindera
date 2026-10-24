//! Training functionality for custom morphological models in PHP.
//!
//! This module provides the PhpTrainer class with static methods for training
//! custom morphological analysis models from annotated corpora.

use std::fs::File;
use std::path::Path;

use ext_php_rs::prelude::*;

use lindera::dictionary::trainer::{
    Corpus, DEFAULT_ELASTIC_NET_L1_RATIO, DEFAULT_ETA, DEFAULT_LAMBDA, Model,
    ParseRegularizationTypeError, RegularizationType, SerializableModel, Trainer, TrainerConfig,
};
use lindera_binding::argument::{Positive, non_negative_float_argument, unit_interval_argument};

use crate::convert::nullable_int_arg;
use crate::error::lindera_value_err;

/// Trainer for morphological analysis models.
///
/// Provides static methods for training and exporting models.
#[php_class]
#[php(name = "Lindera\\Trainer")]
pub struct PhpTrainer;

#[php_impl]
impl PhpTrainer {
    /// Trains a morphological analysis model from an annotated corpus.
    ///
    /// # Arguments
    ///
    /// * `seed` - Seed lexicon file path (CSV format).
    /// * `corpus` - Training corpus file path (annotated text).
    /// * `char_def` - Character definition file path (char.def).
    /// * `unk_def` - Unknown word definition file path (unk.def).
    /// * `feature_def` - Feature definition file path (feature.def).
    /// * `rewrite_def` - Rewrite rule definition file path (rewrite.def).
    /// * `output` - Output model file path.
    /// * `lambda` - Regularization coefficient, finite and >= 0, default:
    ///   1.0; with L2, MeCab's `-c C` is `1 / C`, so 1.0 is MeCab's `-c 1.0`.
    ///   An `int` is widened to a float, as for a PHP `float` parameter.
    /// * `max_iter` - Maximum number of iterations, 1 or more, default: 100.
    /// * `max_threads` - Maximum number of threads, 1 or more (null =
    ///   auto-detect CPU cores).
    /// * `regularization` - Regularization type: `'l1'`, `'l2'` or
    ///   `'elasticnet'` (in any case; also `'elastic_net'` or
    ///   `'elastic-net'`), default: `'l2'`.
    /// * `elastic_net_l1_ratio` - L1 ratio of Elastic Net regularization,
    ///   between 0 and 1, default: 0.5; used only with `'elasticnet'`.
    /// * `eta` - Tolerance of the convergence test, finite and >= 0,
    ///   default: 5e-5: training stops before `max_iter` once the relative
    ///   change of the objective stays below it for 3 iterations in a row; 0
    ///   disables the test.
    ///
    /// `null` selects the default of each optional parameter. A value of
    /// another type throws instead of selecting the default, and a `max_iter`
    /// or `max_threads` below 1, an unknown `regularization`, and a
    /// `lambda`, `elastic_net_l1_ratio` or `eta` outside its range throw a
    /// `ValueError` before any file is read.
    #[allow(clippy::too_many_arguments)]
    pub fn train(
        seed: String,
        corpus: String,
        char_def: String,
        unk_def: String,
        feature_def: String,
        rewrite_def: String,
        output: String,
        lambda: Option<f64>,
        max_iter: Option<i64>,
        max_threads: Option<i64>,
        regularization: Option<String>,
        elastic_net_l1_ratio: Option<f64>,
        eta: Option<f64>,
    ) -> PhpResult<()> {
        // Check the settings before touching any file.
        let regularization = match regularization.as_deref() {
            Some(name) => name
                .parse()
                .map_err(|e: ParseRegularizationTypeError| lindera_value_err(e.to_string()))?,
            None => RegularizationType::default(),
        };
        let lambda_val = non_negative_float_argument("lambda", lambda.unwrap_or(DEFAULT_LAMBDA))
            .map_err(lindera_value_err)?;
        let elastic_net_l1_ratio = unit_interval_argument(
            "elastic_net_l1_ratio",
            elastic_net_l1_ratio.unwrap_or(DEFAULT_ELASTIC_NET_L1_RATIO),
        )
        .map_err(lindera_value_err)?;
        let eta = non_negative_float_argument("eta", eta.unwrap_or(DEFAULT_ETA))
            .map_err(lindera_value_err)?;
        let max_iter_val =
            nullable_int_arg::<Positive<u64>>("max_iter", max_iter)?.map_or(100, |Positive(n)| n);
        let num_threads = nullable_int_arg::<Positive<usize>>("max_threads", max_threads)?
            .map_or_else(num_cpus::get, |Positive(n)| n);

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
                return Err(lindera_value_err(format!(
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
        .map_err(|e| lindera_value_err(format!("Failed to load trainer configuration: {e}")))?;

        // Initialize trainer
        let trainer = Trainer::new(config)
            .map_err(|e| lindera_value_err(format!("Failed to initialize trainer: {e}")))?
            .regularization_cost(lambda_val)
            .regularization_type(regularization, elastic_net_l1_ratio)
            .max_iter(max_iter_val)
            .eta(eta)
            .num_threads(num_threads);

        // Load corpus
        let corpus_file = File::open(corpus_path)
            .map_err(|e| lindera_value_err(format!("Failed to open corpus file: {e}")))?;
        let corpus_data = Corpus::from_reader(corpus_file)
            .map_err(|e| lindera_value_err(format!("Failed to load corpus: {e}")))?;

        println!("Training with {} examples...", corpus_data.len());

        // Train model
        let model = trainer
            .train(corpus_data)
            .map_err(|e| lindera_value_err(format!("Training failed: {e}")))?;

        // Save model
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                lindera_value_err(format!("Failed to create output directory: {e}"))
            })?;
        }

        let mut output_file = File::create(output_path)
            .map_err(|e| lindera_value_err(format!("Failed to create output file: {e}")))?;

        model
            .write_model(&mut output_file)
            .map_err(|e| lindera_value_err(format!("Failed to write model: {e}")))?;

        println!("Model saved to {}", output_path.display());
        Ok(())
    }

    /// Exports dictionary files from a trained model.
    ///
    /// Writes the files `lindera export` writes, with the same method
    /// (`SerializableModel::export`): `lex.csv`, `matrix.def`, `unk.def`,
    /// `char.def`, `feature.def`, `rewrite.def`, `left-id.def` and
    /// `right-id.def`, and `metadata.json` when `metadata` is given.
    ///
    /// # Arguments
    ///
    /// * `model` - Trained model file path (.dat format).
    /// * `output` - Output directory path.
    /// * `metadata` - Optional base metadata.json file to update. A value of
    ///   another type than `string` or `null` throws instead of leaving it out.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `ValueError` on failure.
    pub fn export(model: String, output: String, metadata: Option<String>) -> PhpResult<()> {
        let model_path = Path::new(&model);
        let output_path = Path::new(&output);
        let metadata_path = metadata.as_deref().map(Path::new);

        if !model_path.exists() {
            return Err(lindera_value_err(format!(
                "Model file does not exist: {}",
                model_path.display()
            )));
        }
        if let Some(metadata_path) = metadata_path
            && !metadata_path.exists()
        {
            return Err(lindera_value_err(format!(
                "Metadata file does not exist: {}",
                metadata_path.display()
            )));
        }

        // Load trained model
        let model_file = File::open(model_path)
            .map_err(|e| lindera_value_err(format!("Failed to open model file: {e}")))?;

        let serializable_model: SerializableModel = Model::read_model(model_file)
            .map_err(|e| lindera_value_err(format!("Failed to load model: {e}")))?;

        // Export dictionary files
        let files_created = serializable_model
            .export(output_path, metadata_path)
            .map_err(|e| lindera_value_err(format!("Failed to export dictionary files: {e}")))?;

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
}
