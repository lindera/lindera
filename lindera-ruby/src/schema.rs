//! Dictionary schema definitions.
//!
//! This module provides schema structures that define the format and fields
//! of dictionary entries.

use std::collections::HashMap;

use magnus::prelude::*;
use magnus::{Error, RArray, Ruby, function, method};

use lindera::dictionary::{FieldDefinition, FieldType, Schema};

/// Field type in dictionary schema.
///
/// Defines the type of a field in the dictionary entry.
#[magnus::wrap(class = "Lindera::FieldType", free_immediately, size)]
#[derive(Debug, Clone)]
pub struct RbFieldType {
    /// Internal field type variant.
    inner: RbFieldTypeKind,
}

/// Internal enum for field type kind.
#[derive(Debug, Clone)]
enum RbFieldTypeKind {
    /// Surface form (word text).
    Surface,
    /// Left context ID for morphological analysis.
    LeftContextId,
    /// Right context ID for morphological analysis.
    RightContextId,
    /// Word cost (used in path selection).
    Cost,
    /// Custom field (morphological features).
    Custom,
}

impl RbFieldType {
    /// Returns the string representation of the field type.
    fn to_s(&self) -> &str {
        match self.inner {
            RbFieldTypeKind::Surface => "surface",
            RbFieldTypeKind::LeftContextId => "left_context_id",
            RbFieldTypeKind::RightContextId => "right_context_id",
            RbFieldTypeKind::Cost => "cost",
            RbFieldTypeKind::Custom => "custom",
        }
    }

    /// Returns the inspect representation of the field type.
    fn inspect(&self) -> String {
        format!("#<Lindera::FieldType: {}>", self.to_s())
    }
}

impl From<FieldType> for RbFieldType {
    fn from(field_type: FieldType) -> Self {
        let kind = match field_type {
            FieldType::Surface => RbFieldTypeKind::Surface,
            FieldType::LeftContextId => RbFieldTypeKind::LeftContextId,
            FieldType::RightContextId => RbFieldTypeKind::RightContextId,
            FieldType::Cost => RbFieldTypeKind::Cost,
            FieldType::Custom => RbFieldTypeKind::Custom,
        };
        RbFieldType { inner: kind }
    }
}

impl From<RbFieldType> for FieldType {
    fn from(field_type: RbFieldType) -> Self {
        match field_type.inner {
            RbFieldTypeKind::Surface => FieldType::Surface,
            RbFieldTypeKind::LeftContextId => FieldType::LeftContextId,
            RbFieldTypeKind::RightContextId => FieldType::RightContextId,
            RbFieldTypeKind::Cost => FieldType::Cost,
            RbFieldTypeKind::Custom => FieldType::Custom,
        }
    }
}

/// Field definition in dictionary schema.
///
/// Describes a single field in the dictionary entry format.
#[magnus::wrap(class = "Lindera::FieldDefinition", free_immediately, size)]
#[derive(Debug, Clone)]
pub struct RbFieldDefinition {
    /// Field index in the schema.
    pub index: usize,
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: RbFieldType,
    /// Optional description.
    pub description: Option<String>,
}

impl RbFieldDefinition {
    /// Returns the index of the field.
    fn index(&self) -> usize {
        self.index
    }

    /// Returns the name of the field.
    fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the field type.
    fn field_type(&self) -> RbFieldType {
        self.field_type.clone()
    }

    /// Returns the description of the field.
    fn description(&self) -> Option<String> {
        self.description.clone()
    }

    /// Returns the string representation of the field definition.
    fn to_s(&self) -> String {
        format!("FieldDefinition(index={}, name={})", self.index, self.name)
    }

    /// Returns the inspect representation of the field definition.
    fn inspect(&self) -> String {
        format!(
            "#<Lindera::FieldDefinition: index={}, name='{}', field_type={:?}, description={:?}>",
            self.index, self.name, self.field_type.inner, self.description
        )
    }
}

impl From<FieldDefinition> for RbFieldDefinition {
    fn from(field_def: FieldDefinition) -> Self {
        RbFieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<RbFieldDefinition> for FieldDefinition {
    fn from(field_def: RbFieldDefinition) -> Self {
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
#[magnus::wrap(class = "Lindera::Schema", free_immediately, size)]
#[derive(Debug, Clone)]
pub struct RbSchema {
    /// List of field names.
    pub fields: Vec<String>,
    /// Map from field name to index.
    field_index_map: HashMap<String, usize>,
}

impl RbSchema {
    /// Creates a new `RbSchema` from a list of field names.
    ///
    /// # Arguments
    ///
    /// * `fields` - List of field names.
    ///
    /// # Returns
    ///
    /// A new `RbSchema` instance.
    fn new(fields: Vec<String>) -> Self {
        let mut field_index_map = HashMap::new();
        for (i, field) in fields.iter().enumerate() {
            field_index_map.insert(field.clone(), i);
        }
        Self {
            fields,
            field_index_map,
        }
    }

    /// Creates a default schema for IPADIC-style dictionaries.
    ///
    /// # Returns
    ///
    /// A new `RbSchema` with default IPADIC fields.
    fn create_default() -> Self {
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

    /// Returns the list of field names as a Ruby array.
    fn fields(&self) -> Vec<String> {
        self.fields.clone()
    }

    /// Returns the index of the specified field name.
    ///
    /// # Arguments
    ///
    /// * `field_name` - Name of the field.
    ///
    /// # Returns
    ///
    /// The index of the field, or None if not found.
    fn get_field_index(&self, field_name: String) -> Option<usize> {
        self.field_index_map.get(&field_name).copied()
    }

    /// Returns the number of fields in the schema.
    ///
    /// # Returns
    ///
    /// The number of fields.
    fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns the name of the field at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the field.
    ///
    /// # Returns
    ///
    /// The field name, or None if the index is out of bounds.
    fn get_field_name(&self, index: usize) -> Option<String> {
        self.fields.get(index).cloned()
    }

    /// Returns the custom fields (fields after the first 4 standard fields).
    ///
    /// # Returns
    ///
    /// A list of custom field names.
    fn get_custom_fields(&self) -> Vec<String> {
        if self.fields.len() > 4 {
            self.fields[4..].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Returns all field names.
    ///
    /// # Returns
    ///
    /// A list of all field names.
    fn get_all_fields(&self) -> Vec<String> {
        self.fields.clone()
    }

    /// Returns the field definition for the specified field name.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the field.
    ///
    /// # Returns
    ///
    /// The field definition, or None if not found.
    fn get_field_by_name(&self, name: String) -> Option<RbFieldDefinition> {
        self.field_index_map.get(&name).map(|&index| {
            let field_type = if index < 4 {
                match index {
                    0 => RbFieldType {
                        inner: RbFieldTypeKind::Surface,
                    },
                    1 => RbFieldType {
                        inner: RbFieldTypeKind::LeftContextId,
                    },
                    2 => RbFieldType {
                        inner: RbFieldTypeKind::RightContextId,
                    },
                    3 => RbFieldType {
                        inner: RbFieldTypeKind::Cost,
                    },
                    _ => unreachable!(),
                }
            } else {
                RbFieldType {
                    inner: RbFieldTypeKind::Custom,
                }
            };

            RbFieldDefinition {
                index,
                name: name.clone(),
                field_type,
                description: None,
            }
        })
    }

    /// Validates a CSV record against the schema.
    ///
    /// # Arguments
    ///
    /// * `record` - List of field values.
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or an error if the record is invalid.
    fn validate_record(&self, record: RArray) -> Result<(), Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let values: Vec<String> = record.to_vec()?;

        if values.len() < self.fields.len() {
            return Err(Error::new(
                ruby.exception_arg_error(),
                format!(
                    "CSV row has {} fields but schema requires {} fields",
                    values.len(),
                    self.fields.len()
                ),
            ));
        }

        for (index, field_name) in self.fields.iter().enumerate() {
            if index < values.len() && values[index].trim().is_empty() {
                return Err(Error::new(
                    ruby.exception_arg_error(),
                    format!("Field {field_name} is missing or empty"),
                ));
            }
        }

        Ok(())
    }

    /// Returns the string representation of the schema.
    fn to_s(&self) -> String {
        format!("Schema(fields={})", self.fields.len())
    }

    /// Returns the inspect representation of the schema.
    fn inspect(&self) -> String {
        format!("#<Lindera::Schema: fields={:?}>", self.fields)
    }
}

impl RbSchema {
    /// Internal constructor for use from other modules (not exposed to Ruby).
    pub fn new_internal(fields: Vec<String>) -> Self {
        Self::new(fields)
    }

    /// Internal default constructor for use from other modules (not exposed to Ruby).
    pub fn create_default_internal() -> Self {
        Self::create_default()
    }
}

impl From<RbSchema> for Schema {
    fn from(schema: RbSchema) -> Self {
        Schema::new(schema.fields)
    }
}

impl From<Schema> for RbSchema {
    fn from(schema: Schema) -> Self {
        RbSchema::new(schema.get_all_fields().to_vec())
    }
}

/// Defines Schema, FieldDefinition, and FieldType classes in the given Ruby module.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `module` - Parent Ruby module.
///
/// # Returns
///
/// `Ok(())` on success, or a Magnus `Error` on failure.
pub fn define(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    let field_type_class = module.define_class("FieldType", ruby.class_object())?;
    field_type_class.define_method("to_s", method!(RbFieldType::to_s, 0))?;
    field_type_class.define_method("inspect", method!(RbFieldType::inspect, 0))?;

    let field_def_class = module.define_class("FieldDefinition", ruby.class_object())?;
    field_def_class.define_method("index", method!(RbFieldDefinition::index, 0))?;
    field_def_class.define_method("name", method!(RbFieldDefinition::name, 0))?;
    field_def_class.define_method("field_type", method!(RbFieldDefinition::field_type, 0))?;
    field_def_class.define_method("description", method!(RbFieldDefinition::description, 0))?;
    field_def_class.define_method("to_s", method!(RbFieldDefinition::to_s, 0))?;
    field_def_class.define_method("inspect", method!(RbFieldDefinition::inspect, 0))?;

    let schema_class = module.define_class("Schema", ruby.class_object())?;
    schema_class.define_singleton_method("new", function!(RbSchema::new, 1))?;
    schema_class
        .define_singleton_method("create_default", function!(RbSchema::create_default, 0))?;
    schema_class.define_method("fields", method!(RbSchema::fields, 0))?;
    schema_class.define_method("get_field_index", method!(RbSchema::get_field_index, 1))?;
    schema_class.define_method("field_count", method!(RbSchema::field_count, 0))?;
    schema_class.define_method("get_field_name", method!(RbSchema::get_field_name, 1))?;
    schema_class.define_method("get_custom_fields", method!(RbSchema::get_custom_fields, 0))?;
    schema_class.define_method("get_all_fields", method!(RbSchema::get_all_fields, 0))?;
    schema_class.define_method("get_field_by_name", method!(RbSchema::get_field_by_name, 1))?;
    schema_class.define_method("validate_record", method!(RbSchema::validate_record, 1))?;
    schema_class.define_method("to_s", method!(RbSchema::to_s, 0))?;
    schema_class.define_method("inspect", method!(RbSchema::inspect, 0))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rb_field_type_surface_to_lindera() {
        let rb = RbFieldType {
            inner: RbFieldTypeKind::Surface,
        };
        let lindera: FieldType = rb.into();
        assert!(matches!(lindera, FieldType::Surface));
    }

    #[test]
    fn test_rb_field_type_left_context_id_to_lindera() {
        let rb = RbFieldType {
            inner: RbFieldTypeKind::LeftContextId,
        };
        let lindera: FieldType = rb.into();
        assert!(matches!(lindera, FieldType::LeftContextId));
    }

    #[test]
    fn test_rb_field_type_right_context_id_to_lindera() {
        let rb = RbFieldType {
            inner: RbFieldTypeKind::RightContextId,
        };
        let lindera: FieldType = rb.into();
        assert!(matches!(lindera, FieldType::RightContextId));
    }

    #[test]
    fn test_rb_field_type_cost_to_lindera() {
        let rb = RbFieldType {
            inner: RbFieldTypeKind::Cost,
        };
        let lindera: FieldType = rb.into();
        assert!(matches!(lindera, FieldType::Cost));
    }

    #[test]
    fn test_rb_field_type_custom_to_lindera() {
        let rb = RbFieldType {
            inner: RbFieldTypeKind::Custom,
        };
        let lindera: FieldType = rb.into();
        assert!(matches!(lindera, FieldType::Custom));
    }

    #[test]
    fn test_lindera_field_type_surface_to_rb() {
        let rb: RbFieldType = FieldType::Surface.into();
        assert!(matches!(rb.inner, RbFieldTypeKind::Surface));
    }

    #[test]
    fn test_lindera_field_type_left_context_id_to_rb() {
        let rb: RbFieldType = FieldType::LeftContextId.into();
        assert!(matches!(rb.inner, RbFieldTypeKind::LeftContextId));
    }

    #[test]
    fn test_lindera_field_type_right_context_id_to_rb() {
        let rb: RbFieldType = FieldType::RightContextId.into();
        assert!(matches!(rb.inner, RbFieldTypeKind::RightContextId));
    }

    #[test]
    fn test_lindera_field_type_cost_to_rb() {
        let rb: RbFieldType = FieldType::Cost.into();
        assert!(matches!(rb.inner, RbFieldTypeKind::Cost));
    }

    #[test]
    fn test_lindera_field_type_custom_to_rb() {
        let rb: RbFieldType = FieldType::Custom.into();
        assert!(matches!(rb.inner, RbFieldTypeKind::Custom));
    }

    #[test]
    fn test_rb_schema_new_builds_index_map() {
        let fields = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let schema = RbSchema::new_internal(fields);
        assert_eq!(schema.get_field_index("a".to_string()), Some(0));
        assert_eq!(schema.get_field_index("b".to_string()), Some(1));
        assert_eq!(schema.get_field_index("c".to_string()), Some(2));
        assert_eq!(schema.get_field_index("d".to_string()), None);
    }

    #[test]
    fn test_rb_schema_field_count() {
        let fields = vec!["x".to_string(), "y".to_string()];
        let schema = RbSchema::new_internal(fields);
        assert_eq!(schema.field_count(), 2);
    }

    #[test]
    fn test_rb_schema_get_custom_fields_with_more_than_4() {
        let fields = vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "major_pos".to_string(),
            "reading".to_string(),
        ];
        let schema = RbSchema::new_internal(fields);
        let custom = schema.get_custom_fields();
        assert_eq!(custom, vec!["major_pos", "reading"]);
    }

    #[test]
    fn test_rb_schema_get_custom_fields_with_4_or_fewer() {
        let fields = vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
        ];
        let schema = RbSchema::new_internal(fields);
        let custom = schema.get_custom_fields();
        assert!(custom.is_empty());
    }

    #[test]
    fn test_rb_schema_get_custom_fields_empty() {
        let schema = RbSchema::new_internal(vec![]);
        let custom = schema.get_custom_fields();
        assert!(custom.is_empty());
    }

    #[test]
    fn test_rb_schema_create_default_has_13_fields() {
        let schema = RbSchema::create_default_internal();
        assert_eq!(schema.field_count(), 13);
    }

    #[test]
    fn test_rb_schema_create_default_first_fields() {
        let schema = RbSchema::create_default_internal();
        assert_eq!(schema.fields[0], "surface");
        assert_eq!(schema.fields[1], "left_context_id");
        assert_eq!(schema.fields[2], "right_context_id");
        assert_eq!(schema.fields[3], "cost");
    }

    #[test]
    fn test_rb_schema_to_lindera_schema() {
        let fields = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let rb_schema = RbSchema::new_internal(fields.clone());
        let lindera_schema: Schema = rb_schema.into();
        assert_eq!(lindera_schema.get_all_fields(), &fields);
    }

    #[test]
    fn test_lindera_schema_to_rb_schema() {
        let fields = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let lindera_schema = Schema::new(fields.clone());
        let rb_schema: RbSchema = lindera_schema.into();
        assert_eq!(rb_schema.fields, fields);
        assert_eq!(rb_schema.get_field_index("x".to_string()), Some(0));
        assert_eq!(rb_schema.get_field_index("y".to_string()), Some(1));
        assert_eq!(rb_schema.get_field_index("z".to_string()), Some(2));
    }

    #[test]
    fn test_rb_schema_roundtrip() {
        let fields = vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "reading".to_string(),
        ];
        let rb_schema = RbSchema::new_internal(fields.clone());
        let lindera_schema: Schema = rb_schema.into();
        let back: RbSchema = lindera_schema.into();
        assert_eq!(back.fields, fields);
        assert_eq!(back.field_count(), 5);
    }

    #[test]
    fn test_rb_field_definition_to_lindera() {
        let rb_def = RbFieldDefinition {
            index: 2,
            name: "right_context_id".to_string(),
            field_type: RbFieldType {
                inner: RbFieldTypeKind::RightContextId,
            },
            description: Some("Right context ID".to_string()),
        };
        let lindera_def: FieldDefinition = rb_def.into();
        assert_eq!(lindera_def.index, 2);
        assert_eq!(lindera_def.name, "right_context_id");
        assert!(matches!(lindera_def.field_type, FieldType::RightContextId));
        assert_eq!(
            lindera_def.description,
            Some("Right context ID".to_string())
        );
    }

    #[test]
    fn test_lindera_field_definition_to_rb() {
        let lindera_def = FieldDefinition {
            index: 4,
            name: "major_pos".to_string(),
            field_type: FieldType::Custom,
            description: None,
        };
        let rb_def: RbFieldDefinition = lindera_def.into();
        assert_eq!(rb_def.index, 4);
        assert_eq!(rb_def.name, "major_pos");
        assert!(matches!(rb_def.field_type.inner, RbFieldTypeKind::Custom));
        assert!(rb_def.description.is_none());
    }
}
