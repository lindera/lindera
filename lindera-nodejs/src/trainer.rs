//! Training functionality for custom morphological models.
//!
//! This module provides functions for training custom morphological analysis models
//! from annotated corpora. Requires the `train` feature.

use std::fs::File;
use std::path::Path;

use lindera::dictionary::trainer::{Corpus, Model, SerializableModel, Trainer, TrainerConfig};

use crate::error::to_napi_error;

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
    /// Maximum number of training iterations (default: 100).
    pub max_iter: Option<u32>,
    /// Number of threads (undefined = auto-detect CPU cores).
    pub max_threads: Option<u32>,
}

/// Trains a morphological analysis model from an annotated corpus.
///
/// # Arguments
///
/// * `options` - Training options containing file paths and parameters.
#[napi]
pub fn train(options: TrainOptions) -> napi::Result<()> {
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
    let lambda = options.lambda.unwrap_or(0.01);
    let max_iter = options.max_iter.unwrap_or(100) as u64;
    let num_threads = options
        .max_threads
        .map(|t| t as usize)
        .unwrap_or_else(num_cpus::get);

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
/// # Arguments
///
/// * `options` - Export options containing file paths.
#[napi]
pub fn export_model(options: ExportOptions) -> napi::Result<()> {
    let model_path = Path::new(&options.model);
    let output_path = Path::new(&options.output);

    if !model_path.exists() {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("Model file does not exist: {}", model_path.display()),
        ));
    }

    // Load trained model
    let model_file = File::open(model_path)
        .map_err(|e| to_napi_error(format!("Failed to open model file: {e}")))?;

    let serializable_model: SerializableModel = Model::read_model(model_file)
        .map_err(|e| to_napi_error(format!("Failed to load model: {e}")))?;

    // Create output directory
    std::fs::create_dir_all(output_path)
        .map_err(|e| to_napi_error(format!("Failed to create output directory: {e}")))?;

    // Export dictionary files
    let lexicon_path = output_path.join("lex.csv");
    let connector_path = output_path.join("matrix.def");
    let unk_path = output_path.join("unk.def");
    let char_def_path = output_path.join("char.def");

    // Write lexicon file
    let mut lexicon_file = File::create(&lexicon_path)
        .map_err(|e| to_napi_error(format!("Failed to create lexicon file: {e}")))?;
    serializable_model
        .write_lexicon(&mut lexicon_file)
        .map_err(|e| to_napi_error(format!("Failed to write lexicon: {e}")))?;

    // Write connection matrix
    let mut connector_file = File::create(&connector_path)
        .map_err(|e| to_napi_error(format!("Failed to create connection matrix file: {e}")))?;
    serializable_model
        .write_connection_costs(&mut connector_file)
        .map_err(|e| to_napi_error(format!("Failed to write connection costs: {e}")))?;

    // Write unknown word definitions
    let mut unk_file = File::create(&unk_path)
        .map_err(|e| to_napi_error(format!("Failed to create unknown word file: {e}")))?;
    serializable_model
        .write_unknown_dictionary(&mut unk_file)
        .map_err(|e| to_napi_error(format!("Failed to write unknown dictionary: {e}")))?;

    // Write character definition file
    let mut char_def_file = File::create(&char_def_path)
        .map_err(|e| to_napi_error(format!("Failed to create character definition file: {e}")))?;

    use std::io::Write;
    writeln!(
        char_def_file,
        "# Character definition file generated from trained model"
    )
    .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "# Format: CATEGORY_NAME invoke group length")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "DEFAULT 0 1 0")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "HIRAGANA 1 1 0")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "KATAKANA 1 1 0")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "KANJI 0 0 2")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "ALPHA 1 1 0")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "NUMERIC 1 1 0")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file).map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;

    writeln!(char_def_file, "# Character mappings")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "0x3041..0x3096 HIRAGANA  # Hiragana")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "0x30A1..0x30F6 KATAKANA  # Katakana")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(
        char_def_file,
        "0x4E00..0x9FAF KANJI     # CJK Unified Ideographs"
    )
    .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "0x0030..0x0039 NUMERIC   # ASCII Digits")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "0x0041..0x005A ALPHA     # ASCII Uppercase")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;
    writeln!(char_def_file, "0x0061..0x007A ALPHA     # ASCII Lowercase")
        .map_err(|e| to_napi_error(format!("Failed to write char.def: {e}")))?;

    let mut files_created = vec![
        lexicon_path.clone(),
        connector_path.clone(),
        unk_path.clone(),
        char_def_path.clone(),
    ];

    // Handle metadata.json update if provided
    if let Some(metadata_str) = &options.metadata {
        let metadata_path = Path::new(metadata_str);
        if !metadata_path.exists() {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                format!("Metadata file does not exist: {}", metadata_path.display()),
            ));
        }

        let output_metadata_path = output_path.join("metadata.json");
        let mut metadata_file = File::create(&output_metadata_path)
            .map_err(|e| to_napi_error(format!("Failed to create metadata file: {e}")))?;

        serializable_model
            .update_metadata_json(metadata_path, &mut metadata_file)
            .map_err(|e| to_napi_error(format!("Failed to update metadata: {e}")))?;

        files_created.push(output_metadata_path);
        println!("Updated metadata.json with trained model values");
    }

    println!("Dictionary files exported to: {}", output_path.display());
    println!("Files created:");
    for file in &files_created {
        println!("  - {}", file.display());
    }

    Ok(())
}
