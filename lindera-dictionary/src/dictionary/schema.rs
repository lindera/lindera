use csv::StringRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::LinderaResult;
use crate::error::LinderaErrorKind;

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
    pub name: String,
    pub version: String,
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
            name: String,
            version: String,
            custom_fields: Vec<String>,
        }

        let helper = DictionarySchemaHelper::deserialize(deserializer)?;
        let mut schema = Schema {
            name: helper.name,
            version: helper.version,
            custom_fields: helper.custom_fields,
            field_index_map: None,
        };
        schema.build_index_map();
        Ok(schema)
    }
}

impl Schema {
    /// Create a new dictionary schema
    pub fn new(name: String, version: String, custom_fields: Vec<String>) -> Self {
        let mut schema = Self {
            name,
            version,
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
        map.insert("surface".to_string(), 0);
        map.insert("left_context_id".to_string(), 1);
        map.insert("right_context_id".to_string(), 2);
        map.insert("cost".to_string(), 3);

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
            0 => Some("surface"),
            1 => Some("left_context_id"),
            2 => Some("right_context_id"),
            3 => Some("cost"),
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
            ("surface", CommonField::Surface),
            ("left_context_id", CommonField::LeftContextId),
            ("right_context_id", CommonField::RightContextId),
            ("cost", CommonField::Cost),
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

    /// IPADIC dictionary schema
    pub fn ipadic() -> Self {
        Self::new(
            "IPADIC".to_string(),
            "2.7.0".to_string(),
            vec![
                "major_pos".to_string(),
                "middle_pos".to_string(),
                "small_pos".to_string(),
                "fine_pos".to_string(),
                "conjugation_type".to_string(),
                "conjugation_form".to_string(),
                "base_form".to_string(),
                "reading".to_string(),
                "pronunciation".to_string(),
            ],
        )
    }

    /// IPADIC NEologd dictionary schema
    pub fn ipadic_neologd() -> Self {
        Self::new(
            "IPADIC-NEologd".to_string(),
            "2.7.0".to_string(),
            vec![
                "major_pos".to_string(),
                "middle_pos".to_string(),
                "small_pos".to_string(),
                "fine_pos".to_string(),
                "conjugation_type".to_string(),
                "conjugation_form".to_string(),
                "base_form".to_string(),
                "reading".to_string(),
                "pronunciation".to_string(),
            ],
        )
    }

    /// UniDic dictionary schema (21 fields)
    pub fn unidic() -> Self {
        Self::new(
            "UniDic".to_string(),
            "2.1.2".to_string(),
            vec![
                "major_pos".to_string(),
                "middle_pos".to_string(),
                "small_pos".to_string(),
                "fine_pos".to_string(),
                "conjugation_form".to_string(),
                "conjugation_type".to_string(),
                "lexeme_reading".to_string(),
                "lexeme".to_string(),
                "orthography_appearance".to_string(),
                "pronunciation_appearance".to_string(),
                "orthography_basic".to_string(),
                "pronunciation_basic".to_string(),
                "word_type".to_string(),
                "prefix_form".to_string(),
                "prefix_type".to_string(),
                "suffix_form".to_string(),
                "suffix_type".to_string(),
            ],
        )
    }

    /// CC-CEDICT dictionary schema
    pub fn cc_cedict() -> Self {
        Self::new(
            "CC-CEDICT".to_string(),
            "1.0.0".to_string(),
            vec![
                "major_pos".to_string(),
                "middle_pos".to_string(),
                "small_pos".to_string(),
                "fine_pos".to_string(),
                "pinyin".to_string(),
                "traditional".to_string(),
                "simplified".to_string(),
                "definition".to_string(),
            ],
        )
    }

    /// KO-DIC dictionary schema
    pub fn ko_dic() -> Self {
        Self::new(
            "KO-DIC".to_string(),
            "1.0.0".to_string(),
            vec![
                "pos_tag".to_string(),
                "meaning".to_string(),
                "presence_absence".to_string(),
                "reading".to_string(),
                "type".to_string(),
                "first_pos".to_string(),
                "last_pos".to_string(),
                "expression".to_string(),
            ],
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_schema() {
        let custom_fields = vec!["field1".to_string(), "field2".to_string()];
        let schema = Schema::new("TestDict".to_string(), "1.0.0".to_string(), custom_fields);

        assert_eq!(schema.name, "TestDict");
        assert_eq!(schema.version, "1.0.0");
        assert_eq!(schema.custom_fields.len(), 2);
        assert!(schema.field_index_map.is_some());
    }

    #[test]
    fn test_field_index_lookup() {
        let schema = Schema::ipadic();

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
        let schema = Schema::ipadic();

        assert_eq!(schema.get_field_name(0), Some("surface"));
        assert_eq!(schema.get_field_name(3), Some("cost"));
        assert_eq!(schema.get_field_name(4), Some("major_pos"));
        assert_eq!(schema.get_field_name(12), Some("pronunciation"));
        assert_eq!(schema.get_field_name(13), None);
    }

    #[test]
    fn test_ipadic_schema() {
        let schema = Schema::ipadic();
        assert_eq!(schema.name, "IPADIC");
        assert_eq!(schema.version, "2.7.0");
        assert_eq!(schema.field_count(), 13);
        assert_eq!(schema.custom_fields.len(), 9);
    }

    #[test]
    fn test_unidic_schema() {
        let schema = Schema::unidic();
        assert_eq!(schema.name, "UniDic");
        assert_eq!(schema.version, "2.1.2");
        assert_eq!(schema.field_count(), 21);
        assert_eq!(schema.custom_fields.len(), 17);
    }

    #[test]
    fn test_cc_cedict_schema() {
        let schema = Schema::cc_cedict();
        assert_eq!(schema.name, "CC-CEDICT");
        assert_eq!(schema.version, "1.0.0");
        assert_eq!(schema.field_count(), 12);
        assert_eq!(schema.custom_fields.len(), 8);

        // Check specific fields
        assert_eq!(schema.get_field_index("pinyin"), Some(8));
        assert_eq!(schema.get_field_index("traditional"), Some(9));
        assert_eq!(schema.get_field_index("simplified"), Some(10));
        assert_eq!(schema.get_field_index("definition"), Some(11));
    }

    #[test]
    fn test_ko_dic_schema() {
        let schema = Schema::ko_dic();
        assert_eq!(schema.name, "KO-DIC");
        assert_eq!(schema.version, "1.0.0");
        assert_eq!(schema.field_count(), 12);
        assert_eq!(schema.custom_fields.len(), 8);

        // Check specific fields
        assert_eq!(schema.get_field_index("pos_tag"), Some(4));
        assert_eq!(schema.get_field_index("meaning"), Some(5));
        assert_eq!(schema.get_field_index("expression"), Some(11));
    }

    #[test]
    fn test_common_field_index() {
        let schema = Schema::ipadic();

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
        let schema = Schema::ipadic();
        let record = StringRecord::from(vec!["surface_form", "123", "456", "789", "名詞"]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_common_fields_empty_field() {
        let schema = Schema::ipadic();
        let record = StringRecord::from(vec![
            "", // Empty surface
            "123", "456", "789",
        ]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_common_fields_missing_field() {
        let schema = Schema::ipadic();
        let record = StringRecord::from(vec![
            "surface_form", // Only first field
        ]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_backward_compatibility() {
        let schema = Schema::ipadic();

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
    fn test_detail_fields() {
        let schema = Schema::ipadic();
        let detail_fields = schema.get_detail_fields();
        assert_eq!(detail_fields.len(), 9);
        assert_eq!(detail_fields[0], "major_pos");
        assert_eq!(detail_fields[8], "pronunciation");
    }
}
