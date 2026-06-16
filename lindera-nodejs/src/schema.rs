//! Dictionary schema definitions.
//!
//! This module provides schema structures that define the format and fields
//! of dictionary entries. The field-management logic is delegated to
//! [`lindera_binding_core::CoreSchema`]; this module only adds the napi wrappers.

use lindera::dictionary::{FieldDefinition, FieldType, Schema};
use lindera_binding_core::{CoreFieldDefinition, CoreFieldType, CoreSchema};

use crate::error::to_napi_error;

/// Field type in dictionary schema.
///
/// Defines the type of a field in the dictionary entry.
#[napi(string_enum)]
pub enum JsFieldType {
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

impl From<CoreFieldType> for JsFieldType {
    fn from(field_type: CoreFieldType) -> Self {
        match field_type {
            CoreFieldType::Surface => JsFieldType::Surface,
            CoreFieldType::LeftContextId => JsFieldType::LeftContextId,
            CoreFieldType::RightContextId => JsFieldType::RightContextId,
            CoreFieldType::Cost => JsFieldType::Cost,
            CoreFieldType::Custom => JsFieldType::Custom,
        }
    }
}

impl From<JsFieldType> for CoreFieldType {
    fn from(field_type: JsFieldType) -> Self {
        match field_type {
            JsFieldType::Surface => CoreFieldType::Surface,
            JsFieldType::LeftContextId => CoreFieldType::LeftContextId,
            JsFieldType::RightContextId => CoreFieldType::RightContextId,
            JsFieldType::Cost => CoreFieldType::Cost,
            JsFieldType::Custom => CoreFieldType::Custom,
        }
    }
}

impl From<FieldType> for JsFieldType {
    fn from(field_type: FieldType) -> Self {
        JsFieldType::from(CoreFieldType::from(field_type))
    }
}

impl From<JsFieldType> for FieldType {
    fn from(field_type: JsFieldType) -> Self {
        FieldType::from(CoreFieldType::from(field_type))
    }
}

/// Field definition in dictionary schema.
///
/// Describes a single field in the dictionary entry format.
#[napi(object)]
pub struct JsFieldDefinition {
    /// Field index in the record.
    pub index: u32,
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: JsFieldType,
    /// Optional description of the field.
    pub description: Option<String>,
}

impl From<CoreFieldDefinition> for JsFieldDefinition {
    fn from(field_def: CoreFieldDefinition) -> Self {
        JsFieldDefinition {
            index: field_def.index as u32,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<JsFieldDefinition> for CoreFieldDefinition {
    fn from(field_def: JsFieldDefinition) -> Self {
        CoreFieldDefinition {
            index: field_def.index as usize,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<FieldDefinition> for JsFieldDefinition {
    fn from(field_def: FieldDefinition) -> Self {
        JsFieldDefinition::from(CoreFieldDefinition::from(field_def))
    }
}

impl From<JsFieldDefinition> for FieldDefinition {
    fn from(field_def: JsFieldDefinition) -> Self {
        FieldDefinition::from(CoreFieldDefinition::from(field_def))
    }
}

/// Dictionary schema definition.
///
/// A thin napi wrapper over [`lindera_binding_core::CoreSchema`], which owns the
/// field storage, the name-to-index map, and the field lookups.
#[napi(js_name = "Schema")]
pub struct JsSchema {
    /// The backing binding-core schema.
    inner: CoreSchema,
}

#[napi]
impl JsSchema {
    /// Creates a new schema with the specified field names.
    ///
    /// # Arguments
    ///
    /// * `fields` - Array of field name strings.
    #[napi(constructor)]
    pub fn new(fields: Vec<String>) -> Self {
        Self {
            inner: CoreSchema::new(fields),
        }
    }

    /// Creates a default schema matching the IPADIC format (13 fields).
    ///
    /// # Returns
    ///
    /// A schema with the standard IPADIC field definitions.
    #[napi(factory)]
    pub fn create_default() -> Self {
        Self {
            inner: CoreSchema::create_default(),
        }
    }

    /// Returns the field names in the schema.
    #[napi(getter)]
    pub fn fields(&self) -> Vec<String> {
        self.inner.fields().to_vec()
    }

    /// Returns the index of the specified field name.
    ///
    /// # Arguments
    ///
    /// * `field_name` - Name of the field to look up.
    ///
    /// # Returns
    ///
    /// The zero-based index of the field, or `undefined` if not found.
    #[napi]
    pub fn get_field_index(&self, field_name: String) -> Option<u32> {
        self.inner.get_field_index(&field_name).map(|i| i as u32)
    }

    /// Returns the total number of fields in the schema.
    #[napi]
    pub fn field_count(&self) -> u32 {
        self.inner.field_count() as u32
    }

    /// Returns the field name at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index.
    ///
    /// # Returns
    ///
    /// The field name, or `undefined` if the index is out of range.
    #[napi]
    pub fn get_field_name(&self, index: u32) -> Option<String> {
        self.inner
            .get_field_name(index as usize)
            .map(str::to_string)
    }

    /// Returns the custom fields (index 4 and above).
    ///
    /// # Returns
    ///
    /// An array of custom field names.
    #[napi]
    pub fn get_custom_fields(&self) -> Vec<String> {
        self.inner.get_custom_fields().to_vec()
    }

    /// Returns all field names in the schema.
    ///
    /// # Returns
    ///
    /// An array of all field names.
    #[napi]
    pub fn get_all_fields(&self) -> Vec<String> {
        self.inner.fields().to_vec()
    }

    /// Returns the field definition for the specified field name.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the field to look up.
    ///
    /// # Returns
    ///
    /// The field definition, or `undefined` if not found.
    #[napi]
    pub fn get_field_by_name(&self, name: String) -> Option<JsFieldDefinition> {
        self.inner
            .get_field_by_name(&name)
            .map(JsFieldDefinition::from)
    }

    /// Validates that a CSV record matches the schema.
    ///
    /// # Arguments
    ///
    /// * `record` - Array of field values to validate.
    #[napi]
    pub fn validate_record(&self, record: Vec<String>) -> napi::Result<()> {
        self.inner.validate_record(&record).map_err(to_napi_error)
    }
}

impl From<CoreSchema> for JsSchema {
    fn from(schema: CoreSchema) -> Self {
        JsSchema { inner: schema }
    }
}

impl From<JsSchema> for CoreSchema {
    fn from(schema: JsSchema) -> Self {
        schema.inner
    }
}

impl From<JsSchema> for Schema {
    fn from(schema: JsSchema) -> Self {
        schema.inner.into()
    }
}

impl From<Schema> for JsSchema {
    fn from(schema: Schema) -> Self {
        JsSchema {
            inner: CoreSchema::from(schema),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_field_type_to_field_type_all_variants() {
        assert!(matches!(
            FieldType::from(JsFieldType::Surface),
            FieldType::Surface
        ));
        assert!(matches!(
            FieldType::from(JsFieldType::Custom),
            FieldType::Custom
        ));
    }

    #[test]
    fn test_field_type_to_js_field_type_all_variants() {
        assert!(matches!(
            JsFieldType::from(FieldType::Surface),
            JsFieldType::Surface
        ));
        assert!(matches!(
            JsFieldType::from(FieldType::Custom),
            JsFieldType::Custom
        ));
    }

    #[test]
    fn test_js_schema_new_builds_index_map() {
        let schema = JsSchema::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(schema.get_field_index("a".to_string()), Some(0));
        assert_eq!(schema.get_field_index("b".to_string()), Some(1));
        assert_eq!(schema.get_field_index("c".to_string()), Some(2));
    }

    #[test]
    fn test_js_schema_get_field_index_not_found() {
        let schema = JsSchema::new(vec!["x".to_string()]);
        assert_eq!(schema.get_field_index("y".to_string()), None);
    }

    #[test]
    fn test_js_schema_field_count() {
        let schema = JsSchema::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(schema.field_count(), 3);
    }

    #[test]
    fn test_js_schema_get_field_name() {
        let schema = JsSchema::new(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(schema.get_field_name(0), Some("a".to_string()));
        assert_eq!(schema.get_field_name(9), None);
    }

    #[test]
    fn test_js_schema_get_custom_fields() {
        let schema = JsSchema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "pos1".to_string(),
            "pos2".to_string(),
        ]);
        let custom = schema.get_custom_fields();
        assert_eq!(custom, vec!["pos1".to_string(), "pos2".to_string()]);
    }

    #[test]
    fn test_js_schema_get_custom_fields_no_custom() {
        let schema = JsSchema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
        ]);
        assert!(schema.get_custom_fields().is_empty());
    }

    #[test]
    fn test_js_schema_create_default() {
        let schema = JsSchema::create_default();
        assert_eq!(schema.field_count(), 13);
        assert_eq!(schema.get_field_index("surface".to_string()), Some(0));
        assert_eq!(schema.fields()[5], "middle_pos");
        assert_eq!(
            schema.get_field_index("pronunciation".to_string()),
            Some(12)
        );
    }

    #[test]
    fn test_js_schema_get_field_by_name() {
        let schema = JsSchema::create_default();
        let surface = schema.get_field_by_name("surface".to_string()).unwrap();
        assert_eq!(surface.index, 0);
        assert!(matches!(surface.field_type, JsFieldType::Surface));

        let custom = schema.get_field_by_name("middle_pos".to_string()).unwrap();
        assert_eq!(custom.index, 5);
        assert!(matches!(custom.field_type, JsFieldType::Custom));

        assert!(schema.get_field_by_name("nope".to_string()).is_none());
    }

    #[test]
    fn test_js_schema_to_lindera_schema_roundtrip() {
        let fields = vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
        ];
        let js_schema = JsSchema::new(fields.clone());
        let lindera_schema: Schema = js_schema.into();
        let roundtripped: JsSchema = lindera_schema.into();
        assert_eq!(roundtripped.field_count(), 5);
        assert_eq!(roundtripped.get_field_index("pos".to_string()), Some(4));
    }

    #[test]
    fn test_lindera_schema_to_js_schema() {
        let lindera_schema = Schema::new(vec!["a".to_string(), "b".to_string()]);
        let js_schema: JsSchema = lindera_schema.into();
        assert_eq!(js_schema.field_count(), 2);
        assert_eq!(js_schema.get_field_index("a".to_string()), Some(0));
    }
}
