use csv::StringRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::LinderaResult;
use crate::error::LinderaErrorKind;

/// Common field name constants
pub const FIELD_SURFACE: &str = "surface";
pub const FIELD_LEFT_CONTEXT_ID: &str = "left_context_id";
pub const FIELD_RIGHT_CONTEXT_ID: &str = "right_context_id";
pub const FIELD_COST: &str = "cost";

/// Default custom field name constants
pub const FIELD_PART_OF_SPEECH: &str = "major_pos";
pub const FIELD_PART_OF_SPEECH_SUBCATEGORY_1: &str = "middle_pos";
pub const FIELD_PART_OF_SPEECH_SUBCATEGORY_2: &str = "small_pos";
pub const FIELD_PART_OF_SPEECH_SUBCATEGORY_3: &str = "fine_pos";
pub const FIELD_CONJUGATION_FORM: &str = "conjugation_type";
pub const FIELD_CONJUGATION_TYPE: &str = "conjugation_form";
pub const FIELD_BASE_FORM: &str = "base_form";
pub const FIELD_READING: &str = "reading";
pub const FIELD_PRONUNCIATION: &str = "pronunciation";

/// Common fields present in all dictionaries (first 4 columns)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommonField {
    Surface = 0,
    LeftContextId = 1,
    RightContextId = 2,
    Cost = 3,
}

/// Dictionary schema that defines the structure of dictionary entries
#[derive(Debug, Clone, Serialize)]
pub struct Schema {
    /// Custom field names (from 4th column onwards)
    pub custom_fields: Vec<String>,
    /// Field name to index mapping for fast lookup
    #[serde(skip)]
    field_index_map: Option<HashMap<String, usize>>,
}

impl<'de> serde::Deserialize<'de> for Schema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DictionarySchemaHelper {
            custom_fields: Vec<String>,
        }

        let helper = DictionarySchemaHelper::deserialize(deserializer)?;
        let mut schema = Schema {
            custom_fields: helper.custom_fields,
            field_index_map: None,
        };
        schema.build_index_map();
        Ok(schema)
    }
}

impl Schema {
    /// Create a new dictionary schema
    pub fn new(custom_fields: Vec<String>) -> Self {
        let mut schema = Self {
            custom_fields,
            field_index_map: None,
        };
        schema.build_index_map();
        schema
    }

    /// Build field name to index mapping
    fn build_index_map(&mut self) {
        let mut map = HashMap::new();

        // Common fields
        map.insert(FIELD_SURFACE.to_string(), 0);
        map.insert(FIELD_LEFT_CONTEXT_ID.to_string(), 1);
        map.insert(FIELD_RIGHT_CONTEXT_ID.to_string(), 2);
        map.insert(FIELD_COST.to_string(), 3);

        // Custom fields
        for (i, field) in self.custom_fields.iter().enumerate() {
            map.insert(field.clone(), i + 4);
        }

        self.field_index_map = Some(map);
    }

    /// Get field index by name
    pub fn get_field_index(&self, field_name: &str) -> Option<usize> {
        self.field_index_map
            .as_ref()
            .and_then(|map| map.get(field_name))
            .copied()
    }

    /// Get total field count
    pub fn field_count(&self) -> usize {
        4 + self.custom_fields.len()
    }

    /// Get field name by index
    pub fn get_field_name(&self, index: usize) -> Option<&str> {
        match index {
            0 => Some(FIELD_SURFACE),
            1 => Some(FIELD_LEFT_CONTEXT_ID),
            2 => Some(FIELD_RIGHT_CONTEXT_ID),
            3 => Some(FIELD_COST),
            n => self.custom_fields.get(n - 4).map(|s| s.as_str()),
        }
    }

    /// Get common field index
    pub fn get_common_field_index(&self, field: CommonField) -> usize {
        field as usize
    }

    /// Get detail fields (index >= 4)
    pub fn get_detail_fields(&self) -> &[String] {
        &self.custom_fields
    }

    /// Validate common fields (first 4 columns)
    pub fn validate_common_fields(&self, row: &StringRecord) -> LinderaResult<()> {
        let common_fields = [
            (FIELD_SURFACE, CommonField::Surface),
            (FIELD_LEFT_CONTEXT_ID, CommonField::LeftContextId),
            (FIELD_RIGHT_CONTEXT_ID, CommonField::RightContextId),
            (FIELD_COST, CommonField::Cost),
        ];

        for (name, field) in &common_fields {
            let index = self.get_common_field_index(*field);
            if index >= row.len() || row[index].trim().is_empty() {
                return Err(LinderaErrorKind::Content
                    .with_error(anyhow::anyhow!("Common field {} is missing or empty", name)));
            }
        }

        Ok(())
    }
}

// Helper methods for backward compatibility
impl Schema {
    /// Find field by name (backward compatibility)
    pub fn get_field_by_name(&self, name: &str) -> Option<FieldDefinition> {
        self.get_field_index(name).map(|index| FieldDefinition {
            index,
            name: name.to_string(),
            field_type: if index < 4 {
                match index {
                    0 => FieldType::Surface,
                    1 => FieldType::LeftContextId,
                    2 => FieldType::RightContextId,
                    3 => FieldType::Cost,
                    _ => unreachable!(),
                }
            } else {
                FieldType::Custom
            },
            description: None,
        })
    }
}

// Backward compatibility types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub index: usize,
    pub name: String,
    pub field_type: FieldType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    Surface,
    LeftContextId,
    RightContextId,
    Cost,
    Custom,
}

impl Default for Schema {
    fn default() -> Self {
        Self::new(vec![
            FIELD_PART_OF_SPEECH.to_string(),
            FIELD_PART_OF_SPEECH_SUBCATEGORY_1.to_string(),
            FIELD_PART_OF_SPEECH_SUBCATEGORY_2.to_string(),
            FIELD_PART_OF_SPEECH_SUBCATEGORY_3.to_string(),
            FIELD_CONJUGATION_FORM.to_string(),
            FIELD_CONJUGATION_TYPE.to_string(),
            FIELD_BASE_FORM.to_string(),
            FIELD_READING.to_string(),
            FIELD_PRONUNCIATION.to_string(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_schema() {
        let custom_fields = vec!["field1".to_string(), "field2".to_string()];
        let schema = Schema::new(custom_fields);

        assert_eq!(schema.custom_fields.len(), 2);
        assert!(schema.field_index_map.is_some());
    }

    #[test]
    fn test_field_index_lookup() {
        let schema = Schema::default();

        // Common fields
        assert_eq!(schema.get_field_index(FIELD_SURFACE), Some(0));
        assert_eq!(schema.get_field_index(FIELD_LEFT_CONTEXT_ID), Some(1));
        assert_eq!(schema.get_field_index(FIELD_RIGHT_CONTEXT_ID), Some(2));
        assert_eq!(schema.get_field_index(FIELD_COST), Some(3));

        // Custom fields
        assert_eq!(schema.get_field_index(FIELD_PART_OF_SPEECH), Some(4));
        assert_eq!(schema.get_field_index(FIELD_BASE_FORM), Some(10));
        assert_eq!(schema.get_field_index(FIELD_PRONUNCIATION), Some(12));

        // Non-existent field
        assert_eq!(schema.get_field_index("nonexistent"), None);
    }

    #[test]
    fn test_field_name_lookup() {
        let schema = Schema::default();

        assert_eq!(schema.get_field_name(0), Some(FIELD_SURFACE));
        assert_eq!(schema.get_field_name(3), Some(FIELD_COST));
        assert_eq!(schema.get_field_name(4), Some(FIELD_PART_OF_SPEECH));
        assert_eq!(schema.get_field_name(12), Some(FIELD_PRONUNCIATION));
        assert_eq!(schema.get_field_name(13), None);
    }

    #[test]
    fn test_default_schema() {
        let schema = Schema::default();
        // Name and version fields have been removed from Schema
        assert_eq!(schema.field_count(), 13);
        assert_eq!(schema.custom_fields.len(), 9);
    }

    #[test]
    fn test_common_field_index() {
        let schema = Schema::default();

        assert_eq!(schema.get_common_field_index(CommonField::Surface), 0);
        assert_eq!(schema.get_common_field_index(CommonField::LeftContextId), 1);
        assert_eq!(
            schema.get_common_field_index(CommonField::RightContextId),
            2
        );
        assert_eq!(schema.get_common_field_index(CommonField::Cost), 3);
    }

    #[test]
    fn test_validate_common_fields_success() {
        let schema = Schema::default();
        let record = StringRecord::from(vec!["surface_form", "123", "456", "789", "名詞"]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_common_fields_empty_field() {
        let schema = Schema::default();
        let record = StringRecord::from(vec![
            "", // Empty surface
            "123", "456", "789",
        ]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_common_fields_missing_field() {
        let schema = Schema::default();
        let record = StringRecord::from(vec![
            "surface_form", // Only first field
        ]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_backward_compatibility() {
        let schema = Schema::default();

        // Test get_field_by_name
        let field = schema.get_field_by_name(FIELD_SURFACE).unwrap();
        assert_eq!(field.index, 0);
        assert_eq!(field.name, FIELD_SURFACE);
        assert_eq!(field.field_type, FieldType::Surface);

        let field = schema.get_field_by_name(FIELD_PART_OF_SPEECH).unwrap();
        assert_eq!(field.index, 4);
        assert_eq!(field.name, FIELD_PART_OF_SPEECH);
        assert_eq!(field.field_type, FieldType::Custom);
    }

    #[test]
    fn test_detail_fields() {
        let schema = Schema::default();
        let detail_fields = schema.get_detail_fields();
        assert_eq!(detail_fields.len(), 9);
        assert_eq!(detail_fields[0], FIELD_PART_OF_SPEECH);
        assert_eq!(detail_fields[8], FIELD_PRONUNCIATION);
    }
}
