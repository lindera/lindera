//! Dictionary schema definitions.
//!
//! This module provides schema structures that define the format and fields
//! of dictionary entries. The field-management logic is delegated to
//! [`lindera_binding_core::CoreSchema`]; this module only adds the PyO3 wrappers.
//!
//! # Examples
//!
//! ```python
//! # Create a custom schema
//! schema = lindera.Schema([
//!     "surface",
//!     "left_context_id",
//!     "right_context_id",
//!     "cost",
//!     "part_of_speech"
//! ])
//!
//! # Use default schema
//! schema = lindera.Schema.create_default()
//!
//! # Access field information
//! index = schema.get_field_index("surface")
//! field = schema.get_field_by_name("part_of_speech")
//! ```

use pyo3::prelude::*;

use lindera::dictionary::{FieldDefinition, FieldType, Schema};
use lindera_binding_core::{CoreFieldDefinition, CoreFieldType, CoreSchema};

use crate::error::to_py_error;

/// Field type in dictionary schema.
///
/// Defines the type of a field in the dictionary entry.
#[pyclass(name = "FieldType", from_py_object)]
#[derive(Debug, Clone)]
pub enum PyFieldType {
    /// Surface form (word text)
    Surface,
    /// Left context ID for morphological analysis
    LeftContextId,
    /// Right context ID for morphological analysis
    RightContextId,
    /// Word cost (used in path selection)
    Cost,
    /// Custom field (morphological features)
    Custom,
}

#[pymethods]
impl PyFieldType {
    fn __str__(&self) -> &str {
        match self {
            PyFieldType::Surface => "surface",
            PyFieldType::LeftContextId => "left_context_id",
            PyFieldType::RightContextId => "right_context_id",
            PyFieldType::Cost => "cost",
            PyFieldType::Custom => "custom",
        }
    }

    fn __repr__(&self) -> String {
        format!("FieldType.{self:?}")
    }
}

impl From<CoreFieldType> for PyFieldType {
    fn from(field_type: CoreFieldType) -> Self {
        match field_type {
            CoreFieldType::Surface => PyFieldType::Surface,
            CoreFieldType::LeftContextId => PyFieldType::LeftContextId,
            CoreFieldType::RightContextId => PyFieldType::RightContextId,
            CoreFieldType::Cost => PyFieldType::Cost,
            CoreFieldType::Custom => PyFieldType::Custom,
        }
    }
}

impl From<PyFieldType> for CoreFieldType {
    fn from(field_type: PyFieldType) -> Self {
        match field_type {
            PyFieldType::Surface => CoreFieldType::Surface,
            PyFieldType::LeftContextId => CoreFieldType::LeftContextId,
            PyFieldType::RightContextId => CoreFieldType::RightContextId,
            PyFieldType::Cost => CoreFieldType::Cost,
            PyFieldType::Custom => CoreFieldType::Custom,
        }
    }
}

impl From<FieldType> for PyFieldType {
    fn from(field_type: FieldType) -> Self {
        PyFieldType::from(CoreFieldType::from(field_type))
    }
}

impl From<PyFieldType> for FieldType {
    fn from(field_type: PyFieldType) -> Self {
        FieldType::from(CoreFieldType::from(field_type))
    }
}

/// Field definition in dictionary schema.
///
/// Describes a single field in the dictionary entry format.
#[pyclass(name = "FieldDefinition", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyFieldDefinition {
    #[pyo3(get)]
    pub index: usize,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub field_type: PyFieldType,
    #[pyo3(get)]
    pub description: Option<String>,
}

#[pymethods]
impl PyFieldDefinition {
    #[new]
    pub fn new(
        index: usize,
        name: String,
        field_type: PyFieldType,
        description: Option<String>,
    ) -> Self {
        Self {
            index,
            name,
            field_type,
            description,
        }
    }

    fn __str__(&self) -> String {
        format!("FieldDefinition(index={}, name={})", self.index, self.name)
    }

    fn __repr__(&self) -> String {
        format!(
            "FieldDefinition(index={}, name='{}', field_type={:?}, description={:?})",
            self.index, self.name, self.field_type, self.description
        )
    }
}

impl From<CoreFieldDefinition> for PyFieldDefinition {
    fn from(field_def: CoreFieldDefinition) -> Self {
        PyFieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<PyFieldDefinition> for CoreFieldDefinition {
    fn from(field_def: PyFieldDefinition) -> Self {
        CoreFieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<FieldDefinition> for PyFieldDefinition {
    fn from(field_def: FieldDefinition) -> Self {
        PyFieldDefinition::from(CoreFieldDefinition::from(field_def))
    }
}

impl From<PyFieldDefinition> for FieldDefinition {
    fn from(field_def: PyFieldDefinition) -> Self {
        FieldDefinition::from(CoreFieldDefinition::from(field_def))
    }
}

/// Dictionary schema definition.
///
/// A thin PyO3 wrapper over [`lindera_binding_core::CoreSchema`], which owns the
/// field storage, the name-to-index map, and the field lookups.
///
/// # Examples
///
/// ```python
/// # Create schema
/// schema = lindera.Schema(["surface", "pos", "reading"])
///
/// # Query field information
/// index = schema.get_field_index("pos")
/// field = schema.get_field_by_name("reading")
/// ```
#[pyclass(name = "Schema", from_py_object)]
#[derive(Debug, Clone)]
pub struct PySchema {
    /// The backing binding-core schema.
    pub inner: CoreSchema,
}

#[pymethods]
impl PySchema {
    #[new]
    pub fn new(fields: Vec<String>) -> Self {
        Self {
            inner: CoreSchema::new(fields),
        }
    }

    #[staticmethod]
    pub fn create_default() -> Self {
        Self {
            inner: CoreSchema::create_default(),
        }
    }

    #[getter]
    pub fn fields(&self) -> Vec<String> {
        self.inner.fields().to_vec()
    }

    pub fn get_field_index(&self, field_name: &str) -> Option<usize> {
        self.inner.get_field_index(field_name)
    }

    pub fn field_count(&self) -> usize {
        self.inner.field_count()
    }

    pub fn get_field_name(&self, index: usize) -> Option<&str> {
        self.inner.get_field_name(index)
    }

    pub fn get_custom_fields(&self) -> Vec<String> {
        self.inner.get_custom_fields().to_vec()
    }

    pub fn get_all_fields(&self) -> Vec<String> {
        self.inner.fields().to_vec()
    }

    pub fn get_field_by_name(&self, name: &str) -> Option<PyFieldDefinition> {
        self.inner
            .get_field_by_name(name)
            .map(PyFieldDefinition::from)
    }

    pub fn validate_record(&self, record: Vec<String>) -> PyResult<()> {
        self.inner.validate_record(&record).map_err(to_py_error)
    }

    fn __str__(&self) -> String {
        format!("Schema(fields={})", self.inner.field_count())
    }

    fn __repr__(&self) -> String {
        format!("Schema(fields={:?})", self.inner.fields())
    }

    fn __len__(&self) -> usize {
        self.inner.field_count()
    }
}

impl From<CoreSchema> for PySchema {
    fn from(schema: CoreSchema) -> Self {
        PySchema { inner: schema }
    }
}

impl From<PySchema> for CoreSchema {
    fn from(schema: PySchema) -> Self {
        schema.inner
    }
}

impl From<PySchema> for Schema {
    fn from(schema: PySchema) -> Self {
        schema.inner.into()
    }
}

impl From<Schema> for PySchema {
    fn from(schema: Schema) -> Self {
        PySchema {
            inner: CoreSchema::from(schema),
        }
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "schema")?;
    m.add_class::<PySchema>()?;
    m.add_class::<PyFieldDefinition>()?;
    m.add_class::<PyFieldType>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lindera::dictionary::{FieldDefinition, FieldType, Schema};

    #[test]
    fn test_pyfieldtype_to_fieldtype_all_variants() {
        for (py, ft) in [
            (PyFieldType::Surface, FieldType::Surface),
            (PyFieldType::LeftContextId, FieldType::LeftContextId),
            (PyFieldType::RightContextId, FieldType::RightContextId),
            (PyFieldType::Cost, FieldType::Cost),
            (PyFieldType::Custom, FieldType::Custom),
        ] {
            let converted: FieldType = py.into();
            assert_eq!(converted, ft);
        }
    }

    #[test]
    fn test_fieldtype_to_pyfieldtype_all_variants() {
        assert!(matches!(
            PyFieldType::from(FieldType::Surface),
            PyFieldType::Surface
        ));
        assert!(matches!(
            PyFieldType::from(FieldType::Custom),
            PyFieldType::Custom
        ));
    }

    #[test]
    fn test_pyfielddefinition_to_fielddefinition() {
        let py_fd = PyFieldDefinition {
            index: 0,
            name: "surface".to_string(),
            field_type: PyFieldType::Surface,
            description: Some("Surface form".to_string()),
        };
        let fd: FieldDefinition = py_fd.into();
        assert_eq!(fd.index, 0);
        assert_eq!(fd.name, "surface");
        assert!(matches!(fd.field_type, FieldType::Surface));
        assert_eq!(fd.description, Some("Surface form".to_string()));
    }

    #[test]
    fn test_fielddefinition_to_pyfielddefinition() {
        let fd = FieldDefinition {
            index: 4,
            name: "pos".to_string(),
            field_type: FieldType::Custom,
            description: None,
        };
        let py_fd: PyFieldDefinition = fd.into();
        assert_eq!(py_fd.index, 4);
        assert_eq!(py_fd.name, "pos");
        assert!(matches!(py_fd.field_type, PyFieldType::Custom));
        assert!(py_fd.description.is_none());
    }

    #[test]
    fn test_pyschema_to_schema() {
        let py_schema = PySchema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
        ]);
        let schema: Schema = py_schema.into();
        let fields = schema.get_all_fields();
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "surface");
        assert_eq!(fields[4], "pos");
    }

    #[test]
    fn test_schema_to_pyschema() {
        let schema = Schema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
        ]);
        let py_schema: PySchema = schema.into();
        assert_eq!(py_schema.fields().len(), 4);
        assert_eq!(py_schema.fields()[0], "surface");
    }

    #[test]
    fn test_pyschema_index_and_name_lookups() {
        let schema = PySchema::new(vec![
            "surface".to_string(),
            "pos".to_string(),
            "reading".to_string(),
        ]);
        assert_eq!(schema.get_field_index("surface"), Some(0));
        assert_eq!(schema.get_field_index("reading"), Some(2));
        assert_eq!(schema.get_field_index("nonexistent"), None);
        assert_eq!(schema.get_field_name(1), Some("pos"));
        assert_eq!(schema.get_field_name(9), None);
        assert_eq!(schema.field_count(), 3);
    }

    #[test]
    fn test_pyschema_custom_fields() {
        let schema = PySchema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "major_pos".to_string(),
            "reading".to_string(),
        ]);
        let custom = schema.get_custom_fields();
        assert_eq!(custom, ["major_pos", "reading"]);
    }

    #[test]
    fn test_pyschema_create_default() {
        let schema = PySchema::create_default();
        assert_eq!(schema.field_count(), 13);
        assert_eq!(schema.fields()[0], "surface");
        assert_eq!(schema.fields()[5], "middle_pos");
        assert_eq!(schema.fields()[12], "pronunciation");
        assert_eq!(schema.get_field_index("cost"), Some(3));
    }

    #[test]
    fn test_pyschema_get_field_by_name() {
        let schema = PySchema::create_default();
        let surface = schema.get_field_by_name("surface").unwrap();
        assert_eq!(surface.index, 0);
        assert!(matches!(surface.field_type, PyFieldType::Surface));

        let custom = schema.get_field_by_name("major_pos").unwrap();
        assert_eq!(custom.index, 4);
        assert!(matches!(custom.field_type, PyFieldType::Custom));

        assert!(schema.get_field_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_pyschema_validate_record() {
        let schema = PySchema::new(vec!["surface".to_string(), "reading".to_string()]);
        assert!(
            schema
                .validate_record(vec!["x".to_string(), "y".to_string()])
                .is_ok()
        );
        assert!(schema.validate_record(vec!["x".to_string()]).is_err());
    }

    #[test]
    fn test_pyschema_roundtrip() {
        let fields = vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
        ];
        let py_schema = PySchema::new(fields.clone());
        let schema: Schema = py_schema.into();
        let roundtripped: PySchema = schema.into();
        assert_eq!(roundtripped.fields(), fields);
    }
}
