//! Dictionary schema definitions for PHP.
//!
//! This module provides schema structures that define the format and fields
//! of dictionary entries. The field-management logic is delegated to
//! [`lindera_binding_core::CoreSchema`]; this module only adds the ext-php-rs wrappers.

use ext_php_rs::prelude::*;

use lindera::dictionary::{FieldDefinition, FieldType, Schema};
use lindera_binding_core::{CoreFieldDefinition, CoreSchema};

use crate::error::lindera_value_err;

/// Field type in dictionary schema.
///
/// Defines the type of a field in the dictionary entry.
/// Accepts: "surface", "left_context_id", "right_context_id", "cost", "custom".
#[php_class]
#[php(name = "Lindera\\FieldType")]
pub struct PhpFieldType {
    /// The field type name.
    value: String,
}

#[php_impl]
impl PhpFieldType {
    /// Creates a new FieldType instance.
    ///
    /// # Arguments
    ///
    /// * `value` - Field type string.
    ///
    /// # Returns
    ///
    /// A new FieldType instance.
    pub fn __construct(value: String) -> PhpResult<Self> {
        match value.as_str() {
            "surface" | "left_context_id" | "right_context_id" | "cost" | "custom" => {
                Ok(Self { value })
            }
            _ => Err(lindera_value_err(format!(
                "Invalid field type: {value}. Must be one of: surface, left_context_id, right_context_id, cost, custom"
            ))),
        }
    }

    /// Returns the field type name.
    ///
    /// # Returns
    ///
    /// The field type string.
    #[php(getter)]
    pub fn value(&self) -> String {
        self.value.clone()
    }

    /// Returns a string representation of the field type.
    ///
    /// # Returns
    ///
    /// The field type name.
    pub fn __to_string(&self) -> String {
        self.value.clone()
    }
}

impl PhpFieldType {
    /// Creates a PhpFieldType from a FieldType value.
    ///
    /// # Arguments
    ///
    /// * `ft` - The FieldType to convert.
    ///
    /// # Returns
    ///
    /// A new PhpFieldType.
    pub fn from_field_type(ft: &FieldType) -> Self {
        let value = match ft {
            FieldType::Surface => "surface",
            FieldType::LeftContextId => "left_context_id",
            FieldType::RightContextId => "right_context_id",
            FieldType::Cost => "cost",
            FieldType::Custom => "custom",
        };
        Self {
            value: value.to_string(),
        }
    }

    /// Converts this PhpFieldType to a FieldType.
    ///
    /// # Returns
    ///
    /// The corresponding FieldType.
    pub fn to_field_type(&self) -> FieldType {
        match self.value.as_str() {
            "surface" => FieldType::Surface,
            "left_context_id" => FieldType::LeftContextId,
            "right_context_id" => FieldType::RightContextId,
            "cost" => FieldType::Cost,
            _ => FieldType::Custom,
        }
    }
}

/// Field definition in dictionary schema.
///
/// Describes a single field in the dictionary entry format.
#[php_class]
#[php(name = "Lindera\\FieldDefinition")]
pub struct PhpFieldDefinition {
    /// Field index in the schema.
    index: usize,
    /// Field name.
    name: String,
    /// Field type.
    field_type: String,
    /// Optional field description.
    description: Option<String>,
}

#[php_impl]
impl PhpFieldDefinition {
    /// Creates a new FieldDefinition instance.
    ///
    /// # Arguments
    ///
    /// * `index` - Field index.
    /// * `name` - Field name.
    /// * `field_type` - Field type string.
    /// * `description` - Optional description.
    ///
    /// # Returns
    ///
    /// A new FieldDefinition instance.
    pub fn __construct(
        index: i64,
        name: String,
        field_type: String,
        description: Option<String>,
    ) -> Self {
        Self {
            index: index as usize,
            name,
            field_type,
            description,
        }
    }

    /// Returns the field index.
    ///
    /// # Returns
    ///
    /// The index value.
    #[php(getter)]
    pub fn index(&self) -> i64 {
        self.index as i64
    }

    /// Returns the field name.
    ///
    /// # Returns
    ///
    /// The name string.
    #[php(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the field type.
    ///
    /// # Returns
    ///
    /// The field type string.
    #[php(getter)]
    pub fn field_type(&self) -> String {
        self.field_type.clone()
    }

    /// Returns the field description.
    ///
    /// # Returns
    ///
    /// The description string or null.
    #[php(getter)]
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    /// Returns a string representation.
    ///
    /// # Returns
    ///
    /// A string describing the field definition.
    pub fn __to_string(&self) -> String {
        format!(
            "FieldDefinition(index={}, name='{}', field_type='{}')",
            self.index, self.name, self.field_type
        )
    }
}

impl From<CoreFieldDefinition> for PhpFieldDefinition {
    fn from(fd: CoreFieldDefinition) -> Self {
        let field_type: FieldType = fd.field_type.into();
        Self {
            index: fd.index,
            name: fd.name,
            field_type: PhpFieldType::from_field_type(&field_type).value,
            description: fd.description,
        }
    }
}

impl From<FieldDefinition> for PhpFieldDefinition {
    fn from(fd: FieldDefinition) -> Self {
        let field_type = PhpFieldType::from_field_type(&fd.field_type);
        Self {
            index: fd.index,
            name: fd.name,
            field_type: field_type.value,
            description: fd.description,
        }
    }
}

impl From<PhpFieldDefinition> for FieldDefinition {
    fn from(fd: PhpFieldDefinition) -> Self {
        let ft = PhpFieldType {
            value: fd.field_type,
        };
        FieldDefinition {
            index: fd.index,
            name: fd.name,
            field_type: ft.to_field_type(),
            description: fd.description,
        }
    }
}

/// Dictionary schema definition.
///
/// A thin ext-php-rs wrapper over [`lindera_binding_core::CoreSchema`], which owns
/// the field storage, the name-to-index map, and the field lookups.
#[php_class]
#[php(name = "Lindera\\Schema")]
#[derive(Clone)]
pub struct PhpSchema {
    /// The backing binding-core schema.
    inner: CoreSchema,
}

#[php_impl]
impl PhpSchema {
    /// Creates a new Schema instance.
    ///
    /// # Arguments
    ///
    /// * `fields` - List of field name strings.
    ///
    /// # Returns
    ///
    /// A new Schema instance.
    pub fn __construct(fields: Vec<String>) -> Self {
        Self {
            inner: CoreSchema::new(fields),
        }
    }

    /// Creates a default IPADIC 13-field schema.
    ///
    /// # Returns
    ///
    /// A Schema with the default 13 fields.
    pub fn create_default() -> Self {
        Self {
            inner: CoreSchema::create_default(),
        }
    }

    /// Returns the list of all field names.
    ///
    /// # Returns
    ///
    /// A list of field name strings.
    #[php(getter)]
    pub fn fields(&self) -> Vec<String> {
        self.inner.fields().to_vec()
    }

    /// Returns the index of a field by name.
    ///
    /// # Arguments
    ///
    /// * `field_name` - Name of the field.
    ///
    /// # Returns
    ///
    /// The field index, or -1 if not found.
    pub fn get_field_index(&self, field_name: String) -> i64 {
        self.inner
            .get_field_index(&field_name)
            .map(|i| i as i64)
            .unwrap_or(-1)
    }

    /// Returns the total number of fields.
    ///
    /// # Returns
    ///
    /// The field count.
    pub fn field_count(&self) -> i64 {
        self.inner.field_count() as i64
    }

    /// Returns the field name at the given index.
    ///
    /// # Arguments
    ///
    /// * `index` - Field index.
    ///
    /// # Returns
    ///
    /// The field name, or null if index is out of bounds.
    pub fn get_field_name(&self, index: i64) -> Option<String> {
        self.inner
            .get_field_name(index as usize)
            .map(str::to_string)
    }

    /// Returns custom fields (fields after the first 4 standard fields).
    ///
    /// # Returns
    ///
    /// A list of custom field name strings.
    pub fn get_custom_fields(&self) -> Vec<String> {
        self.inner.get_custom_fields().to_vec()
    }

    /// Returns all field names.
    ///
    /// # Returns
    ///
    /// A list of all field name strings.
    pub fn get_all_fields(&self) -> Vec<String> {
        self.inner.fields().to_vec()
    }

    /// Returns a field definition by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Field name to look up.
    ///
    /// # Returns
    ///
    /// A FieldDefinition instance, or null if not found.
    pub fn get_field_by_name(&self, name: String) -> Option<PhpFieldDefinition> {
        self.inner
            .get_field_by_name(&name)
            .map(PhpFieldDefinition::from)
    }

    /// Validates a CSV record against this schema.
    ///
    /// # Arguments
    ///
    /// * `record` - List of field values.
    ///
    /// # Returns
    ///
    /// Nothing on success, throws on validation failure.
    pub fn validate_record(&self, record: Vec<String>) -> PhpResult<()> {
        self.inner
            .validate_record(&record)
            .map_err(lindera_value_err)
    }

    /// Returns a string representation.
    ///
    /// # Returns
    ///
    /// A string describing the schema.
    pub fn __to_string(&self) -> String {
        format!("Schema(fields={})", self.inner.field_count())
    }
}

impl From<PhpSchema> for Schema {
    fn from(schema: PhpSchema) -> Self {
        schema.inner.into()
    }
}

impl From<Schema> for PhpSchema {
    fn from(schema: Schema) -> Self {
        PhpSchema {
            inner: CoreSchema::from(schema),
        }
    }
}

impl From<CoreSchema> for PhpSchema {
    fn from(schema: CoreSchema) -> Self {
        PhpSchema { inner: schema }
    }
}

impl From<PhpSchema> for CoreSchema {
    fn from(schema: PhpSchema) -> Self {
        schema.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lindera::dictionary::Schema;

    #[test]
    fn test_phpschema_create_default_has_13_fields() {
        let schema = PhpSchema::create_default();
        assert_eq!(schema.fields().len(), 13);
        assert_eq!(schema.fields()[0], "surface");
        assert_eq!(schema.fields()[5], "pos_detail_1");
        assert_eq!(schema.fields()[12], "pronunciation");
    }

    #[test]
    fn test_phpschema_get_field_index() {
        let schema = PhpSchema::__construct(vec!["surface".to_string(), "cost".to_string()]);
        assert_eq!(schema.get_field_index("surface".to_string()), 0);
        assert_eq!(schema.get_field_index("cost".to_string()), 1);
        assert_eq!(schema.get_field_index("nonexistent".to_string()), -1);
    }

    #[test]
    fn test_phpschema_get_field_by_name() {
        let schema = PhpSchema::create_default();
        let surface = schema.get_field_by_name("surface".to_string()).unwrap();
        assert_eq!(surface.index, 0);
        assert_eq!(surface.field_type, "surface");

        let custom = schema
            .get_field_by_name("pos_detail_1".to_string())
            .unwrap();
        assert_eq!(custom.index, 5);
        assert_eq!(custom.field_type, "custom");

        assert!(schema.get_field_by_name("nope".to_string()).is_none());
    }

    #[test]
    fn test_phpschema_roundtrip() {
        let fields = vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
        ];
        let php_schema = PhpSchema::__construct(fields.clone());
        let schema: Schema = php_schema.into();
        let roundtripped: PhpSchema = schema.into();
        assert_eq!(roundtripped.fields(), fields);
    }

    #[test]
    fn test_phpschema_get_custom_fields() {
        let schema = PhpSchema::__construct(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "major_pos".to_string(),
            "reading".to_string(),
        ]);
        let custom = schema.get_custom_fields();
        assert_eq!(custom.len(), 2);
        assert_eq!(custom[0], "major_pos");
        assert_eq!(custom[1], "reading");
    }
}
