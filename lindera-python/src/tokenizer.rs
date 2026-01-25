//! Tokenizer implementation for morphological analysis.
//!
//! This module provides a builder pattern for creating tokenizers and the tokenizer itself.
//!
//! # Examples
//!
//! ```python
//! # Create a tokenizer with custom configuration
//! tokenizer = (lindera.TokenizerBuilder()
//!     .set_mode("normal")
//!     .append_token_filter("japanese_stop_tags", {"tags": ["助詞"]})
//!     .build())
//!
//! # Tokenize text
//! tokens = tokenizer.tokenize("すもももももももものうち")
//! ```

use std::path::Path;
use std::str::FromStr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::{Tokenizer, TokenizerBuilder};

use crate::dictionary::{PyDictionary, PyUserDictionary};
use crate::token::PyToken;
use crate::util::pydict_to_value;

pub type PyDictRef<'a> = &'a Bound<'a, PyDict>;

/// Builder for creating a `Tokenizer` with custom configuration.
///
/// The builder pattern allows for fluent configuration of tokenizer parameters including
/// dictionaries, modes, and filter pipelines.
///
/// # Examples
///
/// ```python
/// builder = lindera.TokenizerBuilder()
/// builder.set_mode("normal")
/// builder.set_dictionary("/path/to/dict")
/// tokenizer = builder.build()
/// ```
#[pyclass(name = "TokenizerBuilder")]
pub struct PyTokenizerBuilder {
    pub inner: TokenizerBuilder,
}

#[pymethods]
impl PyTokenizerBuilder {
    /// Creates a new `TokenizerBuilder` with default configuration.
    ///
    /// # Returns
    ///
    /// A new instance of `TokenizerBuilder`.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder cannot be initialized.
    #[new]
    #[pyo3(signature = ())]
    fn new() -> PyResult<Self> {
        let inner = TokenizerBuilder::new().map_err(|err| {
            PyValueError::new_err(format!("Failed to create TokenizerBuilder: {err}"))
        })?;

        Ok(Self { inner })
    }

    /// Loads configuration from a file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the configuration file.
    ///
    /// # Returns
    ///
    /// A new `TokenizerBuilder` with the loaded configuration.
    #[pyo3(signature = (file_path))]
    #[allow(clippy::wrong_self_convention)]
    fn from_file(&self, file_path: &str) -> PyResult<Self> {
        let inner = TokenizerBuilder::from_file(Path::new(file_path)).map_err(|err| {
            PyValueError::new_err(format!("Failed to load config from file: {err}"))
        })?;

        Ok(Self { inner })
    }

    /// Sets the tokenization mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - Mode string ("normal" or "decompose").
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    #[pyo3(signature = (mode))]
    fn set_mode<'a>(mut slf: PyRefMut<'a, Self>, mode: &str) -> PyResult<PyRefMut<'a, Self>> {
        let m = Mode::from_str(mode)
            .map_err(|err| PyValueError::new_err(format!("Failed to create mode: {err}")))?;

        slf.inner.set_segmenter_mode(&m);

        Ok(slf)
    }

    /// Sets the dictionary path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the dictionary directory.
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    #[pyo3(signature = (path))]
    fn set_dictionary<'a>(mut slf: PyRefMut<'a, Self>, path: &str) -> PyResult<PyRefMut<'a, Self>> {
        slf.inner.set_segmenter_dictionary(path);

        Ok(slf)
    }

    /// Sets the user dictionary URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - URI to the user dictionary.
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    #[pyo3(signature = (uri))]
    fn set_user_dictionary<'a>(
        mut slf: PyRefMut<'a, Self>,
        uri: &str,
    ) -> PyResult<PyRefMut<'a, Self>> {
        slf.inner.set_segmenter_user_dictionary(uri);
        Ok(slf)
    }

    /// Sets whether to keep whitespace in tokenization results.
    ///
    /// # Arguments
    ///
    /// * `keep_whitespace` - If true, whitespace tokens will be included in results.
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    #[pyo3(signature = (keep_whitespace))]
    fn set_keep_whitespace<'a>(
        mut slf: PyRefMut<'a, Self>,
        keep_whitespace: bool,
    ) -> PyResult<PyRefMut<'a, Self>> {
        slf.inner.set_segmenter_keep_whitespace(keep_whitespace);
        Ok(slf)
    }

    /// Appends a character filter to the filter pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - Type of character filter to add.
    /// * `args` - Optional dictionary of filter arguments.
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    #[pyo3(signature = (kind, args=None))]
    fn append_character_filter<'a>(
        mut slf: PyRefMut<'a, Self>,
        kind: &str,
        args: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        let filter_args = if let Some(dict) = args {
            pydict_to_value(dict)?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        slf.inner.append_character_filter(kind, &filter_args);

        Ok(slf)
    }

    /// Appends a token filter to the filter pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - Type of token filter to add.
    /// * `args` - Optional dictionary of filter arguments.
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    #[pyo3(signature = (kind, args=None))]
    fn append_token_filter<'a>(
        mut slf: PyRefMut<'a, Self>,
        kind: &str,
        args: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        let filter_args = if let Some(dict) = args {
            pydict_to_value(dict)?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        slf.inner.append_token_filter(kind, &filter_args);

        Ok(slf)
    }

    /// Builds the tokenizer with the configured settings.
    ///
    /// # Returns
    ///
    /// A configured `Tokenizer` instance ready for use.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokenizer cannot be built with the current configuration.
    #[pyo3(signature = ())]
    fn build(&self) -> PyResult<PyTokenizer> {
        let tokenizer = self
            .inner
            .build()
            .map_err(|err| PyValueError::new_err(format!("Failed to build tokenizer: {err}")))?;

        Ok(PyTokenizer { inner: tokenizer })
    }
}

/// Tokenizer for performing morphological analysis.
///
/// The tokenizer processes text and returns tokens with their morphological features.
///
/// # Examples
///
/// ```python
/// # Using TokenizerBuilder (recommended)
/// tokenizer = lindera.TokenizerBuilder().build()
///
/// # Or create directly with a dictionary
/// dictionary = lindera.load_dictionary("ipadic")
/// tokenizer = lindera.Tokenizer(dictionary, mode="normal")
/// ```
#[pyclass(name = "Tokenizer")]
pub struct PyTokenizer {
    inner: Tokenizer,
}

#[pymethods]
impl PyTokenizer {
    /// Creates a new tokenizer with the given dictionary and mode.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - Dictionary to use for tokenization.
    /// * `mode` - Tokenization mode ("normal" or "decompose"). Default: "normal".
    /// * `user_dictionary` - Optional user dictionary for custom words.
    ///
    /// # Returns
    ///
    /// A new `Tokenizer` instance.
    #[new]
    #[pyo3(signature = (dictionary, mode="normal", user_dictionary=None))]
    fn new(
        dictionary: PyDictionary,
        mode: &str,
        user_dictionary: Option<PyUserDictionary>,
    ) -> PyResult<Self> {
        let m = Mode::from_str(mode)
            .map_err(|err| PyValueError::new_err(format!("Failed to create mode: {err}")))?;

        let dict = dictionary.inner;
        let user_dict = user_dictionary.map(|d| d.inner);

        let segmenter = Segmenter::new(m, dict, user_dict);
        let tokenizer = Tokenizer::new(segmenter);

        Ok(Self { inner: tokenizer })
    }

    /// Tokenizes the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to tokenize.
    ///
    /// # Returns
    ///
    /// A list of Token objects containing morphological features.
    ///
    /// # Errors
    ///
    /// Returns an error if tokenization fails.
    #[pyo3(signature = (text))]
    fn tokenize(&self, text: &str) -> PyResult<Vec<PyToken>> {
        // Tokenize the processed text
        let tokens = self
            .inner
            .tokenize(text)
            .map_err(|err| PyValueError::new_err(format!("Failed to tokenize text: {err}")))?;

        // Convert to PyToken objects
        let py_tokens: Vec<PyToken> = tokens.into_iter().map(PyToken::from_token).collect();

        Ok(py_tokens)
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "tokenizer")?;
    m.add_class::<PyTokenizerBuilder>()?;
    m.add_class::<PyTokenizer>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}
