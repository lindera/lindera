//! Dictionary schema definitions.
//!
//! This module provides schema structures that define the format and fields
//! of dictionary entries.

use std::collections::HashMap;

use lindera::dictionary::{FieldDefinition, FieldType, Schema};

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

impl From<FieldType> for JsFieldType {
    fn from(field_type: FieldType) -> Self {
        match field_type {
            FieldType::Surface => JsFieldType::Surface,
            FieldType::LeftContextId => JsFieldType::LeftContextId,
            FieldType::RightContextId => JsFieldType::RightContextId,
            FieldType::Cost => JsFieldType::Cost,
            FieldType::Custom => JsFieldType::Custom,
        }
    }
}

impl From<JsFieldType> for FieldType {
    fn from(field_type: JsFieldType) -> Self {
        match field_type {
            JsFieldType::Surface => FieldType::Surface,
            JsFieldType::LeftContextId => FieldType::LeftContextId,
            JsFieldType::RightContextId => FieldType::RightContextId,
            JsFieldType::Cost => FieldType::Cost,
            JsFieldType::Custom => FieldType::Custom,
        }
    }
}

/// Field definition in dictionary schema.
///
/// Describes a single field in the dictionary entry format.
#[napi(object)]
#[derive(Clone)]
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

impl From<FieldDefinition> for JsFieldDefinition {
    fn from(field_def: FieldDefinition) -> Self {
        JsFieldDefinition {
            index: field_def.index as u32,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<JsFieldDefinition> for FieldDefinition {
    fn from(field_def: JsFieldDefinition) -> Self {
        FieldDefinition {
            index: field_def.index as usize,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

/// Dictionary schema definition.
///
/// Defines the structure and fields of dictionary entries.
#[napi(js_name = "Schema")]
pub struct JsSchema {
    /// Field names in the schema.
    fields: Vec<String>,
    /// Index map for fast field lookup.
    field_index_map: HashMap<String, usize>,
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
        let field_index_map = fields
            .iter()
            .enumerate()
            .map(|(i, f)| (f.clone(), i))
            .collect();
        Self {
            fields,
            field_index_map,
        }
    }

    /// Creates a default schema matching the IPADIC format (13 fields).
    ///
    /// # Returns
    ///
    /// A schema with the standard IPADIC field definitions.
    #[napi(factory)]
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

    /// Returns the field names in the schema.
    #[napi(getter)]
    pub fn fields(&self) -> Vec<String> {
        self.fields.clone()
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
        self.field_index_map.get(&field_name).map(|&i| i as u32)
    }

    /// Returns the total number of fields in the schema.
    #[napi]
    pub fn field_count(&self) -> u32 {
        self.fields.len() as u32
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
        self.fields.get(index as usize).cloned()
    }

    /// Returns the custom fields (index 4 and above).
    ///
    /// # Returns
    ///
    /// An array of custom field names.
    #[napi]
    pub fn get_custom_fields(&self) -> Vec<String> {
        if self.fields.len() > 4 {
            self.fields[4..].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Returns all field names in the schema.
    ///
    /// # Returns
    ///
    /// An array of all field names.
    #[napi]
    pub fn get_all_fields(&self) -> Vec<String> {
        self.fields.clone()
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
        self.field_index_map.get(&name).map(|&index| {
            let field_type = match index {
                0 => JsFieldType::Surface,
                1 => JsFieldType::LeftContextId,
                2 => JsFieldType::RightContextId,
                3 => JsFieldType::Cost,
                _ => JsFieldType::Custom,
            };

            JsFieldDefinition {
                index: index as u32,
                name,
                field_type,
                description: None,
            }
        })
    }

    /// Validates that a CSV record matches the schema.
    ///
    /// # Arguments
    ///
    /// * `record` - Array of field values to validate.
    #[napi]
    pub fn validate_record(&self, record: Vec<String>) -> napi::Result<()> {
        if record.len() < self.fields.len() {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                format!(
                    "CSV row has {} fields but schema requires {} fields",
                    record.len(),
                    self.fields.len()
                ),
            ));
        }

        for (index, field_name) in self.fields.iter().enumerate() {
            if index < record.len() && record[index].trim().is_empty() {
                return Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    format!("Field {field_name} is missing or empty"),
                ));
            }
        }

        Ok(())
    }
}

impl From<JsSchema> for Schema {
    fn from(schema: JsSchema) -> Self {
        Schema::new(schema.fields)
    }
}

impl From<Schema> for JsSchema {
    fn from(schema: Schema) -> Self {
        JsSchema::new(schema.get_all_fields().to_vec())
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
            FieldType::from(JsFieldType::LeftContextId),
            FieldType::LeftContextId
        ));
        assert!(matches!(
            FieldType::from(JsFieldType::RightContextId),
            FieldType::RightContextId
        ));
        assert!(matches!(
            FieldType::from(JsFieldType::Cost),
            FieldType::Cost
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
            JsFieldType::from(FieldType::LeftContextId),
            JsFieldType::LeftContextId
        ));
        assert!(matches!(
            JsFieldType::from(FieldType::RightContextId),
            JsFieldType::RightContextId
        ));
        assert!(matches!(
            JsFieldType::from(FieldType::Cost),
            JsFieldType::Cost
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
    fn test_js_schema_field_count_empty() {
        let schema = JsSchema::new(vec![]);
        assert_eq!(schema.field_count(), 0);
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
        let custom = schema.get_custom_fields();
        assert!(custom.is_empty());
    }

    #[test]
    fn test_js_schema_get_custom_fields_fewer_than_4() {
        let schema = JsSchema::new(vec!["surface".to_string()]);
        let custom = schema.get_custom_fields();
        assert!(custom.is_empty());
    }

    #[test]
    fn test_js_schema_create_default_has_13_fields() {
        let schema = JsSchema::create_default();
        assert_eq!(schema.field_count(), 13);
    }

    #[test]
    fn test_js_schema_create_default_field_names() {
        let schema = JsSchema::create_default();
        assert_eq!(schema.get_field_index("surface".to_string()), Some(0));
        assert_eq!(
            schema.get_field_index("pronunciation".to_string()),
            Some(12)
        );
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
        assert_eq!(js_schema.get_field_index("b".to_string()), Some(1));
    }
}
