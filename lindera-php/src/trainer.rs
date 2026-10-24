//! Training functionality for custom morphological models in PHP.
//!
//! This module provides the PhpTrainer class with static methods for training
//! custom morphological analysis models from annotated corpora.

use std::fs::File;
use std::path::Path;

use ext_php_rs::prelude::*;

use lindera::dictionary::trainer::{
    Corpus, DEFAULT_LAMBDA, Model, SerializableModel, Trainer, TrainerConfig,
};

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
    /// * `lambda` - L2 regularization coefficient (>= 0), default: 1.0,
    ///   MeCab's `-c 1.0`. An `int` is widened to a float, as for a PHP
    ///   `float` parameter. The bindings train with L2 regularization; the
    ///   type cannot be chosen yet (#1262).
    /// * `max_iter` - Maximum number of iterations, default: 100.
    /// * `max_threads` - Maximum number of threads (null = auto-detect CPU cores).
    ///
    /// A value of another type for `lambda`, `max_iter` or `max_threads`
    /// throws instead of selecting the default, and a negative `max_iter` or
    /// `max_threads` throws a `ValueError` before any file is read.
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
    ) -> PhpResult<()> {
        // Check the settings before touching any file.
        let lambda_val = lambda.unwrap_or(DEFAULT_LAMBDA);
        let max_iter_val = nullable_int_arg("max_iter", max_iter)?.unwrap_or(100);
        let num_threads =
            nullable_int_arg("max_threads", max_threads)?.unwrap_or_else(num_cpus::get);

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
            .max_iter(max_iter_val)
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
