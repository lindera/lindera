use csv::StringRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::LinderaResult;
use crate::error::LinderaErrorKind;

/// Dictionary schema that defines the structure of dictionary entries
#[derive(Debug, Clone, Serialize)]
pub struct Schema {
    /// All field names including common fields (surface, left_context_id, right_context_id, cost, ...)
    pub fields: Vec<String>,
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
            fields: Vec<String>,
        }

        let helper = DictionarySchemaHelper::deserialize(deserializer)?;
        let mut schema = Schema {
            fields: helper.fields,
            field_index_map: None,
        };
        schema.build_index_map();
        Ok(schema)
    }
}

impl Schema {
    /// Create a new dictionary schema
    pub fn new(fields: Vec<String>) -> Self {
        let mut schema = Self {
            fields,
            field_index_map: None,
        };
        schema.build_index_map();
        schema
    }

    /// Build field name to index mapping
    fn build_index_map(&mut self) {
        let mut map = HashMap::new();

        // All fields
        for (i, field) in self.fields.iter().enumerate() {
            map.insert(field.clone(), i);
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
        self.fields.len()
    }

    /// Get field name by index
    pub fn get_field_name(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(|s| s.as_str())
    }

    /// Get custom fields (index >= 4)
    pub fn get_custom_fields(&self) -> &[String] {
        if self.fields.len() > 4 {
            &self.fields[4..]
        } else {
            &[]
        }
    }

    /// Get all fields
    pub fn get_all_fields(&self) -> &[String] {
        &self.fields
    }

    /// Validate that CSV row has all required fields
    pub fn validate_fields(&self, row: &StringRecord) -> LinderaResult<()> {
        if row.len() < self.fields.len() {
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "CSV row has {} fields but schema requires {} fields",
                row.len(),
                self.fields.len()
            )));
        }

        // Check that required fields are not empty
        for (index, field_name) in self.fields.iter().enumerate() {
            if index < row.len() && row[index].trim().is_empty() {
                return Err(LinderaErrorKind::Content
                    .with_error(anyhow::anyhow!("Field {} is missing or empty", field_name)));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_schema() {
        let fields = vec!["field1".to_string(), "field2".to_string()];
        let schema = Schema::new(fields);

        assert_eq!(schema.fields.len(), 2);
        assert!(schema.field_index_map.is_some());
    }

    #[test]
    fn test_field_index_lookup() {
        let schema = Schema::default();

        // Common fields
        assert_eq!(schema.get_field_index("surface"), Some(0));
        assert_eq!(schema.get_field_index("left_context_id"), Some(1));
        assert_eq!(schema.get_field_index("right_context_id"), Some(2));
        assert_eq!(schema.get_field_index("cost"), Some(3));

        // Custom fields
        assert_eq!(schema.get_field_index("major_pos"), Some(4));
        assert_eq!(schema.get_field_index("base_form"), Some(10));
        assert_eq!(schema.get_field_index("pronunciation"), Some(12));

        // Non-existent field
        assert_eq!(schema.get_field_index("nonexistent"), None);
    }

    #[test]
    fn test_field_name_lookup() {
        let schema = Schema::default();

        assert_eq!(schema.get_field_name(0), Some("surface"));
        assert_eq!(schema.get_field_name(3), Some("cost"));
        assert_eq!(schema.get_field_name(4), Some("major_pos"));
        assert_eq!(schema.get_field_name(12), Some("pronunciation"));
        assert_eq!(schema.get_field_name(13), None);
    }

    #[test]
    fn test_default_schema() {
        let schema = Schema::default();
        // All fields including common fields
        assert_eq!(schema.field_count(), 13);
        assert_eq!(schema.fields.len(), 13);
        assert_eq!(schema.get_custom_fields().len(), 9);
    }

    #[test]
    fn test_field_access() {
        let schema = Schema::default();

        assert_eq!(schema.get_field_index("surface"), Some(0));
        assert_eq!(schema.get_field_index("left_context_id"), Some(1));
        assert_eq!(schema.get_field_index("right_context_id"), Some(2));
        assert_eq!(schema.get_field_index("cost"), Some(3));
    }

    #[test]
    fn test_validate_fields_success() {
        let schema = Schema::default();
        let record = StringRecord::from(vec![
            "surface_form",
            "123",
            "456",
            "789",
            "名詞",
            "一般",
            "*",
            "*",
            "*",
            "*",
            "surface_form",
            "読み",
            "発音",
        ]);

        let result = schema.validate_fields(&record);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fields_empty_field() {
        let schema = Schema::default();
        let record = StringRecord::from(vec![
            "", // Empty surface
            "123",
            "456",
            "789",
            "名詞",
            "一般",
            "*",
            "*",
            "*",
            "*",
            "surface_form",
            "読み",
            "発音",
        ]);

        let result = schema.validate_fields(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_fields_missing_field() {
        let schema = Schema::default();
        let record = StringRecord::from(vec![
            "surface_form", // Only first field
        ]);

        let result = schema.validate_fields(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_backward_compatibility() {
        let schema = Schema::default();

        // Test get_field_by_name
        let field = schema.get_field_by_name("surface").unwrap();
        assert_eq!(field.index, 0);
        assert_eq!(field.name, "surface");
        assert_eq!(field.field_type, FieldType::Surface);

        let field = schema.get_field_by_name("major_pos").unwrap();
        assert_eq!(field.index, 4);
        assert_eq!(field.name, "major_pos");
        assert_eq!(field.field_type, FieldType::Custom);
    }

    #[test]
    fn test_custom_fields() {
        let schema = Schema::default();
        let custom_fields = schema.get_custom_fields();
        assert_eq!(custom_fields.len(), 9);
        assert_eq!(custom_fields[0], "major_pos");
        assert_eq!(custom_fields[8], "pronunciation");
    }
}
