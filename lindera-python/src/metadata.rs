//! Dictionary metadata configuration.
//!
//! This module provides structures for configuring dictionary metadata, including
//! character encodings and schema definitions. The defaults and schema wiring are
//! delegated to [`lindera_binding_core::CoreMetadata`]; this module only adds the
//! PyO3 wrappers.
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
//! )
//!
//! # Load metadata from JSON
//! metadata = lindera.Metadata.from_json_file("metadata.json")
//! ```

use std::collections::HashMap;

use pyo3::prelude::*;

use lindera::dictionary::Metadata;
use lindera_binding_core::CoreMetadata;

use crate::schema::PySchema;

/// Dictionary metadata configuration.
///
/// A thin PyO3 wrapper over [`lindera_binding_core::CoreMetadata`], which owns
/// the default values and the schema wiring.
///
/// # Fields
///
/// * `name` - Dictionary name
/// * `encoding` - Character encoding (default: "UTF-8")
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
    /// The backing binding-core metadata.
    pub inner: CoreMetadata,
}

#[pymethods]
impl PyMetadata {
    #[new]
    #[pyo3(signature = (name=None, encoding=None, default_word_cost=None, default_left_context_id=None, default_right_context_id=None, default_field_value=None, flexible_csv=None, skip_invalid_cost_or_id=None, normalize_details=None, dictionary_schema=None, user_dictionary_schema=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Option<String>,
        encoding: Option<String>,
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
            inner: CoreMetadata::new(
                name,
                encoding,
                default_word_cost,
                default_left_context_id,
                default_right_context_id,
                default_field_value,
                flexible_csv,
                skip_invalid_cost_or_id,
                normalize_details,
                dictionary_schema.map(Into::into),
                user_dictionary_schema.map(Into::into),
            ),
        }
    }

    #[staticmethod]
    pub fn create_default() -> Self {
        PyMetadata {
            inner: CoreMetadata::create_default(),
        }
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
        &self.inner.name
    }

    #[setter]
    pub fn set_name(&mut self, name: String) {
        self.inner.name = name;
    }

    #[getter]
    pub fn encoding(&self) -> &str {
        &self.inner.encoding
    }

    #[setter]
    pub fn set_encoding(&mut self, encoding: String) {
        self.inner.encoding = encoding;
    }

    #[getter]
    pub fn default_word_cost(&self) -> i16 {
        self.inner.default_word_cost
    }

    #[setter]
    pub fn set_default_word_cost(&mut self, cost: i16) {
        self.inner.default_word_cost = cost;
    }

    #[getter]
    pub fn default_left_context_id(&self) -> u16 {
        self.inner.default_left_context_id
    }

    #[setter]
    pub fn set_default_left_context_id(&mut self, id: u16) {
        self.inner.default_left_context_id = id;
    }

    #[getter]
    pub fn default_right_context_id(&self) -> u16 {
        self.inner.default_right_context_id
    }

    #[setter]
    pub fn set_default_right_context_id(&mut self, id: u16) {
        self.inner.default_right_context_id = id;
    }

    #[getter]
    pub fn default_field_value(&self) -> &str {
        &self.inner.default_field_value
    }

    #[setter]
    pub fn set_default_field_value(&mut self, value: String) {
        self.inner.default_field_value = value;
    }

    #[getter]
    pub fn flexible_csv(&self) -> bool {
        self.inner.flexible_csv
    }

    #[setter]
    pub fn set_flexible_csv(&mut self, value: bool) {
        self.inner.flexible_csv = value;
    }

    #[getter]
    pub fn skip_invalid_cost_or_id(&self) -> bool {
        self.inner.skip_invalid_cost_or_id
    }

    #[setter]
    pub fn set_skip_invalid_cost_or_id(&mut self, value: bool) {
        self.inner.skip_invalid_cost_or_id = value;
    }

    #[getter]
    pub fn normalize_details(&self) -> bool {
        self.inner.normalize_details
    }

    #[setter]
    pub fn set_normalize_details(&mut self, value: bool) {
        self.inner.normalize_details = value;
    }

    #[getter]
    pub fn dictionary_schema(&self) -> PySchema {
        PySchema::from(self.inner.dictionary_schema.clone())
    }

    #[setter]
    pub fn set_dictionary_schema(&mut self, schema: PySchema) {
        self.inner.dictionary_schema = schema.into();
    }

    #[getter]
    pub fn user_dictionary_schema(&self) -> PySchema {
        PySchema::from(self.inner.user_dictionary_schema.clone())
    }

    #[setter]
    pub fn set_user_dictionary_schema(&mut self, schema: PySchema) {
        self.inner.user_dictionary_schema = schema.into();
    }

    pub fn to_dict(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), self.inner.name.clone());
        dict.insert("encoding".to_string(), self.inner.encoding.clone());
        dict.insert(
            "default_word_cost".to_string(),
            self.inner.default_word_cost.to_string(),
        );
        dict.insert(
            "default_left_context_id".to_string(),
            self.inner.default_left_context_id.to_string(),
        );
        dict.insert(
            "default_right_context_id".to_string(),
            self.inner.default_right_context_id.to_string(),
        );
        dict.insert(
            "default_field_value".to_string(),
            self.inner.default_field_value.clone(),
        );
        dict.insert(
            "flexible_csv".to_string(),
            self.inner.flexible_csv.to_string(),
        );
        dict.insert(
            "skip_invalid_cost_or_id".to_string(),
            self.inner.skip_invalid_cost_or_id.to_string(),
        );
        dict.insert(
            "normalize_details".to_string(),
            self.inner.normalize_details.to_string(),
        );
        dict.insert(
            "dictionary_schema_fields".to_string(),
            self.inner.dictionary_schema.fields().join(","),
        );
        dict.insert(
            "user_dictionary_schema_fields".to_string(),
            self.inner.user_dictionary_schema.fields().join(","),
        );
        dict
    }

    fn __str__(&self) -> String {
        format!(
            "Metadata(name='{}', encoding='{}')",
            self.inner.name, self.inner.encoding,
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "Metadata(name='{}', encoding='{}', schema_fields={})",
            self.inner.name,
            self.inner.encoding,
            self.inner.dictionary_schema.field_count()
        )
    }
}

impl From<PyMetadata> for Metadata {
    fn from(metadata: PyMetadata) -> Self {
        metadata.inner.into()
    }
}

impl From<Metadata> for PyMetadata {
    fn from(metadata: Metadata) -> Self {
        PyMetadata {
            inner: CoreMetadata::from(metadata),
        }
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "metadata")?;
    m.add_class::<PyMetadata>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lindera::dictionary::Metadata;

    #[test]
    fn test_pymetadata_to_metadata() {
        let py_meta = PyMetadata::new(
            Some("test_dict".to_string()),
            Some("EUC-JP".to_string()),
            Some(-5000),
            Some(100),
            Some(200),
            Some("N/A".to_string()),
            Some(true),
            Some(true),
            Some(true),
            None,
            None,
        );
        let meta: Metadata = py_meta.into();
        assert_eq!(meta.name, "test_dict");
        assert_eq!(meta.encoding, "EUC-JP");
        assert_eq!(meta.default_word_cost, -5000);
        assert_eq!(meta.default_left_context_id, 100);
        assert_eq!(meta.default_right_context_id, 200);
        assert_eq!(meta.default_field_value, "N/A");
        assert!(meta.flexible_csv);
        assert!(meta.skip_invalid_cost_or_id);
        assert!(meta.normalize_details);
    }

    #[test]
    fn test_metadata_to_pymetadata() {
        let schema = lindera::dictionary::Schema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
        ]);
        let userdic_schema =
            lindera::dictionary::Schema::new(vec!["surface".to_string(), "reading".to_string()]);
        let meta = Metadata::new(
            "my_dict".to_string(),
            "UTF-8".to_string(),
            -10000,
            1288,
            1288,
            "*".to_string(),
            false,
            false,
            false,
            schema,
            userdic_schema,
        );
        let py_meta: PyMetadata = meta.into();
        assert_eq!(py_meta.name(), "my_dict");
        assert_eq!(py_meta.encoding(), "UTF-8");
        assert_eq!(py_meta.default_word_cost(), -10000);
        assert_eq!(py_meta.default_left_context_id(), 1288);
        assert_eq!(py_meta.default_right_context_id(), 1288);
        assert_eq!(py_meta.default_field_value(), "*");
        assert!(!py_meta.flexible_csv());
        assert!(!py_meta.skip_invalid_cost_or_id());
        assert!(!py_meta.normalize_details());
        assert_eq!(py_meta.dictionary_schema().fields().len(), 4);
        assert_eq!(py_meta.user_dictionary_schema().fields().len(), 2);
    }

    #[test]
    fn test_pymetadata_default_values() {
        let py_meta = PyMetadata::create_default();
        assert_eq!(py_meta.name(), "default");
        assert_eq!(py_meta.encoding(), "UTF-8");
        assert_eq!(py_meta.default_word_cost(), -10000);
        assert_eq!(py_meta.default_left_context_id(), 1288);
        assert_eq!(py_meta.default_right_context_id(), 1288);
        assert_eq!(py_meta.default_field_value(), "*");
        assert!(!py_meta.flexible_csv());
        assert!(!py_meta.skip_invalid_cost_or_id());
        assert!(!py_meta.normalize_details());
        assert_eq!(py_meta.dictionary_schema().field_count(), 13);
        assert_eq!(py_meta.user_dictionary_schema().fields().len(), 3);
    }

    #[test]
    fn test_pymetadata_roundtrip() {
        let py_meta = PyMetadata::new(
            Some("roundtrip".to_string()),
            Some("UTF-8".to_string()),
            Some(-8000),
            Some(500),
            Some(600),
            Some("?".to_string()),
            Some(true),
            Some(false),
            Some(true),
            None,
            None,
        );
        let meta: Metadata = py_meta.into();
        let roundtripped: PyMetadata = meta.into();
        assert_eq!(roundtripped.name(), "roundtrip");
        assert_eq!(roundtripped.encoding(), "UTF-8");
        assert_eq!(roundtripped.default_word_cost(), -8000);
        assert_eq!(roundtripped.default_left_context_id(), 500);
        assert_eq!(roundtripped.default_right_context_id(), 600);
        assert_eq!(roundtripped.default_field_value(), "?");
        assert!(roundtripped.flexible_csv());
        assert!(!roundtripped.skip_invalid_cost_or_id());
        assert!(roundtripped.normalize_details());
    }
}
