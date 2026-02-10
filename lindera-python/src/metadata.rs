//! Dictionary metadata configuration.
//!
//! This module provides structures for configuring dictionary metadata, including
//! compression algorithms, character encodings, and schema definitions.
//!
//! # Examples
//!
//! ```python
//! # Create metadata with default values
//! metadata = lindera.Metadata()
//!
//! # Create metadata with custom values
//! metadata = lindera.Metadata(
//!     name="custom_dict",
//!     encoding="UTF-8",
//!     compress_algorithm=lindera.CompressionAlgorithm.Deflate
//! )
//!
//! # Load metadata from JSON
//! metadata = lindera.Metadata.from_json_file("metadata.json")
//! ```

use std::collections::HashMap;

use pyo3::prelude::*;

use lindera::dictionary::{CompressionAlgorithm, Metadata};

use crate::schema::PySchema;

/// Compression algorithm for dictionary data.
///
/// Determines how dictionary data is compressed when saved to disk.
#[pyclass(name = "CompressionAlgorithm", from_py_object)]
#[derive(Debug, Clone)]
pub enum PyCompressionAlgorithm {
    /// DEFLATE compression algorithm
    Deflate,
    /// Zlib compression algorithm
    Zlib,
    /// Gzip compression algorithm
    Gzip,
    /// No compression (raw data)
    Raw,
}

#[pymethods]
impl PyCompressionAlgorithm {
    fn __str__(&self) -> &str {
        match self {
            PyCompressionAlgorithm::Deflate => "deflate",
            PyCompressionAlgorithm::Zlib => "zlib",
            PyCompressionAlgorithm::Gzip => "gzip",
            PyCompressionAlgorithm::Raw => "raw",
        }
    }

    fn __repr__(&self) -> String {
        format!("CompressionAlgorithm.{self:?}")
    }
}

impl From<PyCompressionAlgorithm> for CompressionAlgorithm {
    fn from(alg: PyCompressionAlgorithm) -> Self {
        match alg {
            PyCompressionAlgorithm::Deflate => CompressionAlgorithm::Deflate,
            PyCompressionAlgorithm::Zlib => CompressionAlgorithm::Zlib,
            PyCompressionAlgorithm::Gzip => CompressionAlgorithm::Gzip,
            PyCompressionAlgorithm::Raw => CompressionAlgorithm::Raw,
        }
    }
}

impl From<CompressionAlgorithm> for PyCompressionAlgorithm {
    fn from(alg: CompressionAlgorithm) -> Self {
        match alg {
            CompressionAlgorithm::Deflate => PyCompressionAlgorithm::Deflate,
            CompressionAlgorithm::Zlib => PyCompressionAlgorithm::Zlib,
            CompressionAlgorithm::Gzip => PyCompressionAlgorithm::Gzip,
            CompressionAlgorithm::Raw => PyCompressionAlgorithm::Raw,
        }
    }
}

/// Dictionary metadata configuration.
///
/// Contains all configuration parameters for building and using dictionaries.
///
/// # Fields
///
/// * `name` - Dictionary name
/// * `encoding` - Character encoding (default: "UTF-8")
/// * `compress_algorithm` - Compression algorithm (default: Deflate)
/// * `default_word_cost` - Default cost for unknown words (default: -10000)
/// * `default_left_context_id` - Default left context ID (default: 1288)
/// * `default_right_context_id` - Default right context ID (default: 1288)
/// * `default_field_value` - Default value for missing fields (default: "*")
/// * `flexible_csv` - Allow flexible CSV parsing (default: false)
/// * `skip_invalid_cost_or_id` - Skip entries with invalid cost/ID (default: false)
/// * `normalize_details` - Normalize morphological details (default: false)
/// * `dictionary_schema` - Schema for main dictionary
/// * `user_dictionary_schema` - Schema for user dictionary
#[pyclass(name = "Metadata", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyMetadata {
    name: String,
    encoding: String,
    compress_algorithm: PyCompressionAlgorithm,
    default_word_cost: i16,
    default_left_context_id: u16,
    default_right_context_id: u16,
    default_field_value: String,
    flexible_csv: bool,
    skip_invalid_cost_or_id: bool,
    normalize_details: bool,
    dictionary_schema: PySchema,
    user_dictionary_schema: PySchema,
}

#[pymethods]
impl PyMetadata {
    #[new]
    #[pyo3(signature = (name=None, encoding=None, compress_algorithm=None, default_word_cost=None, default_left_context_id=None, default_right_context_id=None, default_field_value=None, flexible_csv=None, skip_invalid_cost_or_id=None, normalize_details=None, dictionary_schema=None, user_dictionary_schema=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Option<String>,
        encoding: Option<String>,
        compress_algorithm: Option<PyCompressionAlgorithm>,
        default_word_cost: Option<i16>,
        default_left_context_id: Option<u16>,
        default_right_context_id: Option<u16>,
        default_field_value: Option<String>,
        flexible_csv: Option<bool>,
        skip_invalid_cost_or_id: Option<bool>,
        normalize_details: Option<bool>,
        dictionary_schema: Option<PySchema>,
        user_dictionary_schema: Option<PySchema>,
    ) -> Self {
        PyMetadata {
            name: name.unwrap_or_else(|| "default".to_string()),
            encoding: encoding.unwrap_or_else(|| "UTF-8".to_string()),
            compress_algorithm: compress_algorithm.unwrap_or(PyCompressionAlgorithm::Deflate),
            default_word_cost: default_word_cost.unwrap_or(-10000),
            default_left_context_id: default_left_context_id.unwrap_or(1288),
            default_right_context_id: default_right_context_id.unwrap_or(1288),
            default_field_value: default_field_value.unwrap_or_else(|| "*".to_string()),
            flexible_csv: flexible_csv.unwrap_or(false),
            skip_invalid_cost_or_id: skip_invalid_cost_or_id.unwrap_or(false),
            normalize_details: normalize_details.unwrap_or(false),
            dictionary_schema: dictionary_schema.unwrap_or_else(PySchema::create_default),
            user_dictionary_schema: user_dictionary_schema.unwrap_or_else(|| {
                PySchema::new(vec![
                    "surface".to_string(),
                    "reading".to_string(),
                    "pronunciation".to_string(),
                ])
            }),
        }
    }

    #[staticmethod]
    pub fn create_default() -> Self {
        PyMetadata::new(
            None, None, None, None, None, None, None, None, None, None, None, None,
        )
    }

    #[staticmethod]
    pub fn from_json_file(path: &str) -> PyResult<Self> {
        use std::fs;

        let json_str = fs::read_to_string(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to read file: {e}"))
        })?;

        let metadata: Metadata = serde_json::from_str(&json_str).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to parse JSON: {e}"))
        })?;

        Ok(metadata.into())
    }

    #[getter]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[setter]
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    #[getter]
    pub fn encoding(&self) -> &str {
        &self.encoding
    }

    #[setter]
    pub fn set_encoding(&mut self, encoding: String) {
        self.encoding = encoding;
    }

    #[getter]
    pub fn compress_algorithm(&self) -> PyCompressionAlgorithm {
        self.compress_algorithm.clone()
    }

    #[setter]
    pub fn set_compress_algorithm(&mut self, algorithm: PyCompressionAlgorithm) {
        self.compress_algorithm = algorithm;
    }

    #[getter]
    pub fn default_word_cost(&self) -> i16 {
        self.default_word_cost
    }

    #[setter]
    pub fn set_default_word_cost(&mut self, cost: i16) {
        self.default_word_cost = cost;
    }

    #[getter]
    pub fn default_left_context_id(&self) -> u16 {
        self.default_left_context_id
    }

    #[setter]
    pub fn set_default_left_context_id(&mut self, id: u16) {
        self.default_left_context_id = id;
    }

    #[getter]
    pub fn default_right_context_id(&self) -> u16 {
        self.default_right_context_id
    }

    #[setter]
    pub fn set_default_right_context_id(&mut self, id: u16) {
        self.default_right_context_id = id;
    }

    #[getter]
    pub fn default_field_value(&self) -> &str {
        &self.default_field_value
    }

    #[setter]
    pub fn set_default_field_value(&mut self, value: String) {
        self.default_field_value = value;
    }

    #[getter]
    pub fn flexible_csv(&self) -> bool {
        self.flexible_csv
    }

    #[setter]
    pub fn set_flexible_csv(&mut self, value: bool) {
        self.flexible_csv = value;
    }

    #[getter]
    pub fn skip_invalid_cost_or_id(&self) -> bool {
        self.skip_invalid_cost_or_id
    }

    #[setter]
    pub fn set_skip_invalid_cost_or_id(&mut self, value: bool) {
        self.skip_invalid_cost_or_id = value;
    }

    #[getter]
    pub fn normalize_details(&self) -> bool {
        self.normalize_details
    }

    #[setter]
    pub fn set_normalize_details(&mut self, value: bool) {
        self.normalize_details = value;
    }

    #[getter]
    pub fn dictionary_schema(&self) -> PySchema {
        self.dictionary_schema.clone()
    }

    #[setter]
    pub fn set_dictionary_schema(&mut self, schema: PySchema) {
        self.dictionary_schema = schema;
    }

    #[getter]
    pub fn user_dictionary_schema(&self) -> PySchema {
        self.user_dictionary_schema.clone()
    }

    #[setter]
    pub fn set_user_dictionary_schema(&mut self, schema: PySchema) {
        self.user_dictionary_schema = schema;
    }

    pub fn to_dict(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), self.name.clone());
        dict.insert("encoding".to_string(), self.encoding.clone());
        dict.insert(
            "compress_algorithm".to_string(),
            self.compress_algorithm.__str__().to_string(),
        );
        dict.insert(
            "default_word_cost".to_string(),
            self.default_word_cost.to_string(),
        );
        dict.insert(
            "default_left_context_id".to_string(),
            self.default_left_context_id.to_string(),
        );
        dict.insert(
            "default_right_context_id".to_string(),
            self.default_right_context_id.to_string(),
        );
        dict.insert(
            "default_field_value".to_string(),
            self.default_field_value.clone(),
        );
        dict.insert("flexible_csv".to_string(), self.flexible_csv.to_string());
        dict.insert(
            "skip_invalid_cost_or_id".to_string(),
            self.skip_invalid_cost_or_id.to_string(),
        );
        dict.insert(
            "normalize_details".to_string(),
            self.normalize_details.to_string(),
        );
        dict.insert(
            "dictionary_schema_fields".to_string(),
            self.dictionary_schema.fields.join(","),
        );
        dict.insert(
            "user_dictionary_schema_fields".to_string(),
            self.user_dictionary_schema.fields.join(","),
        );
        dict
    }

    fn __str__(&self) -> String {
        format!(
            "Metadata(name='{}', encoding='{}', compress_algorithm='{}')",
            self.name,
            self.encoding,
            self.compress_algorithm.__str__()
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "Metadata(name='{}', encoding='{}', compress_algorithm={:?}, schema_fields={})",
            self.name,
            self.encoding,
            self.compress_algorithm,
            self.dictionary_schema.field_count()
        )
    }
}

impl From<PyMetadata> for Metadata {
    fn from(metadata: PyMetadata) -> Self {
        Metadata::new(
            metadata.name,
            metadata.encoding,
            metadata.compress_algorithm.into(),
            metadata.default_word_cost,
            metadata.default_left_context_id,
            metadata.default_right_context_id,
            metadata.default_field_value,
            metadata.flexible_csv,
            metadata.skip_invalid_cost_or_id,
            metadata.normalize_details,
            metadata.dictionary_schema.into(),
            metadata.user_dictionary_schema.into(),
        )
    }
}

impl From<Metadata> for PyMetadata {
    fn from(metadata: Metadata) -> Self {
        PyMetadata {
            name: metadata.name,
            encoding: metadata.encoding,
            compress_algorithm: metadata.compress_algorithm.into(),
            default_word_cost: metadata.default_word_cost,
            default_left_context_id: metadata.default_left_context_id,
            default_right_context_id: metadata.default_right_context_id,
            default_field_value: metadata.default_field_value,
            flexible_csv: metadata.flexible_csv,
            skip_invalid_cost_or_id: metadata.skip_invalid_cost_or_id,
            normalize_details: metadata.normalize_details,
            dictionary_schema: metadata.dictionary_schema.into(),
            user_dictionary_schema: metadata.user_dictionary_schema.into(),
        }
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "metadata")?;
    m.add_class::<PyMetadata>()?;
    m.add_class::<PyCompressionAlgorithm>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}
