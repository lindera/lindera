//! # Lindera Python Bindings
//!
//! Python bindings for [Lindera](https://github.com/lindera/lindera), a morphological analysis library for CJK text.
//!
//! Lindera provides high-performance tokenization and morphological analysis for:
//! - Japanese (IPADIC, IPADIC NEologd, UniDic)
//! - Korean (ko-dic)
//! - Chinese (CC-CEDICT)
//!
//! ## Features
//!
//! - **Dictionary management**: Build, load, and use custom dictionaries
//! - **Tokenization**: Multiple tokenization modes (normal, decompose)
//! - **Filters**: Character and token filtering pipeline
//! - **Training**: Train custom morphological models (with `train` feature)
//! - **User dictionaries**: Support for custom user dictionaries
//!
//! ## Examples
//!
//! ```python
//! import lindera
//!
//! # Create a tokenizer
//! tokenizer = lindera.TokenizerBuilder().build()
//!
//! # Tokenize text
//! tokens = tokenizer.tokenize("関西国際空港")
//! for token in tokens:
//!     print(token["text"], token["detail"])
//! ```

pub mod character_filter;
pub mod dictionary;
pub mod error;
pub mod metadata;
pub mod mode;
pub mod schema;
pub mod segmenter;
pub mod token;
pub mod token_filter;
pub mod tokenizer;
pub mod util;

#[cfg(feature = "train")]
pub mod trainer;

use pyo3::prelude::*;

/// Returns the version of the lindera-python package.
#[pyfunction]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Python module definition for lindera.
#[pymodule]
fn lindera(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register submodules
    tokenizer::register(m)?;
    dictionary::register(m)?;
    token::register(m)?;
    mode::register(m)?;
    metadata::register(m)?;
    schema::register(m)?;
    segmenter::register(m)?;
    character_filter::register(m)?;
    token_filter::register(m)?;
    error::register(m)?;

    #[cfg(feature = "train")]
    {
        // For trainer, we can implement register similarly or just keeping it flat for now if complex
        // Let's assume we want lindera.trainer.train() etc.
        let py = m.py();
        let trainer_mod = PyModule::new(py, "trainer")?;
        trainer_mod.add_function(wrap_pyfunction!(crate::trainer::train, &trainer_mod)?)?;
        trainer_mod.add_function(wrap_pyfunction!(crate::trainer::export, &trainer_mod)?)?;
        m.add_submodule(&trainer_mod)?;
    }

    m.add_function(wrap_pyfunction!(version, m)?)?;

    // --- Backward compatibility aliases (top-level) ---
    // Classes
    m.add("Tokenizer", m.getattr("tokenizer")?.getattr("Tokenizer")?)?;
    m.add(
        "TokenizerBuilder",
        m.getattr("tokenizer")?.getattr("TokenizerBuilder")?,
    )?;
    m.add(
        "Dictionary",
        m.getattr("dictionary")?.getattr("Dictionary")?,
    )?;
    m.add(
        "UserDictionary",
        m.getattr("dictionary")?.getattr("UserDictionary")?,
    )?;
    m.add("Token", m.getattr("token")?.getattr("Token")?)?;
    m.add("Mode", m.getattr("mode")?.getattr("Mode")?)?;
    m.add("Penalty", m.getattr("mode")?.getattr("Penalty")?)?;
    m.add("Metadata", m.getattr("metadata")?.getattr("Metadata")?)?;
    m.add(
        "CompressionAlgorithm",
        m.getattr("metadata")?.getattr("CompressionAlgorithm")?,
    )?;
    m.add("Schema", m.getattr("schema")?.getattr("Schema")?)?;
    m.add(
        "FieldDefinition",
        m.getattr("schema")?.getattr("FieldDefinition")?,
    )?;
    m.add("FieldType", m.getattr("schema")?.getattr("FieldType")?)?;
    m.add("LinderaError", m.getattr("error")?.getattr("LinderaError")?)?;

    // Functions
    m.add(
        "load_dictionary",
        m.getattr("dictionary")?.getattr("load_dictionary")?,
    )?;
    m.add(
        "load_user_dictionary",
        m.getattr("dictionary")?.getattr("load_user_dictionary")?,
    )?;
    m.add(
        "build_dictionary",
        m.getattr("dictionary")?.getattr("build_dictionary")?,
    )?;
    m.add(
        "build_user_dictionary",
        m.getattr("dictionary")?.getattr("build_user_dictionary")?,
    )?;

    #[cfg(feature = "train")]
    {
        m.add("train", m.getattr("trainer")?.getattr("train")?)?;
        m.add("export", m.getattr("trainer")?.getattr("export")?)?;
    }

    Ok(())
}
