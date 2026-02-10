//! Dictionary schema definitions.
//!
//! This module provides schema structures that define the format and fields
//! of dictionary entries.
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

use std::collections::HashMap;

use pyo3::prelude::*;

use lindera::dictionary::{FieldDefinition, FieldType, Schema};

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

impl From<FieldType> for PyFieldType {
    fn from(field_type: FieldType) -> Self {
        match field_type {
            FieldType::Surface => PyFieldType::Surface,
            FieldType::LeftContextId => PyFieldType::LeftContextId,
            FieldType::RightContextId => PyFieldType::RightContextId,
            FieldType::Cost => PyFieldType::Cost,
            FieldType::Custom => PyFieldType::Custom,
        }
    }
}

impl From<PyFieldType> for FieldType {
    fn from(field_type: PyFieldType) -> Self {
        match field_type {
            PyFieldType::Surface => FieldType::Surface,
            PyFieldType::LeftContextId => FieldType::LeftContextId,
            PyFieldType::RightContextId => FieldType::RightContextId,
            PyFieldType::Cost => FieldType::Cost,
            PyFieldType::Custom => FieldType::Custom,
        }
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

impl From<FieldDefinition> for PyFieldDefinition {
    fn from(field_def: FieldDefinition) -> Self {
        PyFieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<PyFieldDefinition> for FieldDefinition {
    fn from(field_def: PyFieldDefinition) -> Self {
        FieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

/// Dictionary schema definition.
///
/// Defines the structure and fields of dictionary entries.
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
    #[pyo3(get)]
    pub fields: Vec<String>,
    field_index_map: Option<HashMap<String, usize>>,
}

#[pymethods]
impl PySchema {
    #[new]
    pub fn new(fields: Vec<String>) -> Self {
        let mut schema = Self {
            fields,
            field_index_map: None,
        };
        schema.build_index_map();
        schema
    }

    #[staticmethod]
    pub fn create_default() -> Self {
        Self::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "major_pos".to_string(),
            "middle_pos".to_string(),
            "small_pos".to_string(),
            "fine_pos".to_string(),
            "conjugation_type".to_string(),
            "conjugation_form".to_string(),
            "base_form".to_string(),
            "reading".to_string(),
            "pronunciation".to_string(),
        ])
    }

    pub fn get_field_index(&self, field_name: &str) -> Option<usize> {
        self.field_index_map
            .as_ref()
            .and_then(|map| map.get(field_name))
            .copied()
    }

    pub fn field_count(&self) -> usize {
        self.get_all_fields().len()
    }

    pub fn get_field_name(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(|s| s.as_str())
    }

    pub fn get_custom_fields(&self) -> Vec<String> {
        if self.fields.len() > 4 {
            self.fields[4..].to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn get_all_fields(&self) -> Vec<String> {
        self.fields.clone()
    }

    pub fn get_field_by_name(&self, name: &str) -> Option<PyFieldDefinition> {
        self.get_field_index(name).map(|index| {
            let field_type = if index < 4 {
                match index {
                    0 => PyFieldType::Surface,
                    1 => PyFieldType::LeftContextId,
                    2 => PyFieldType::RightContextId,
                    3 => PyFieldType::Cost,
                    _ => unreachable!(),
                }
            } else {
                PyFieldType::Custom
            };

            PyFieldDefinition {
                index,
                name: name.to_string(),
                field_type,
                description: None,
            }
        })
    }

    pub fn validate_record(&self, record: Vec<String>) -> PyResult<()> {
        if record.len() < self.fields.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "CSV row has {} fields but schema requires {} fields",
                record.len(),
                self.fields.len()
            )));
        }

        // Check that required fields are not empty
        for (index, field_name) in self.fields.iter().enumerate() {
            if index < record.len() && record[index].trim().is_empty() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Field {field_name} is missing or empty"
                )));
            }
        }

        Ok(())
    }

    fn __str__(&self) -> String {
        format!("Schema(fields={})", self.fields.len())
    }

    fn __repr__(&self) -> String {
        format!("Schema(fields={:?})", self.fields)
    }

    fn __len__(&self) -> usize {
        self.fields.len()
    }
}

impl PySchema {
    fn build_index_map(&mut self) {
        let mut map = HashMap::new();
        for (i, field) in self.fields.iter().enumerate() {
            map.insert(field.clone(), i);
        }
        self.field_index_map = Some(map);
    }
}

impl From<PySchema> for Schema {
    fn from(schema: PySchema) -> Self {
        Schema::new(schema.fields)
    }
}

impl From<Schema> for PySchema {
    fn from(schema: Schema) -> Self {
        PySchema::new(schema.get_all_fields().to_vec())
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
