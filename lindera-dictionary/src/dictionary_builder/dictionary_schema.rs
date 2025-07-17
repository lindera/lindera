use crate::LinderaResult;
use crate::error::LinderaErrorKind;
use csv::StringRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionarySchema {
    pub name: String,
    pub version: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub index: usize,
    pub name: String,
    pub field_type: FieldType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    // Common fields (first 4 columns shared by all dictionaries)
    Surface,
    LeftContextId,
    RightContextId,
    Cost,
    // Dictionary-specific fields
    Custom,
}

impl DictionarySchema {
    /// IPADIC dictionary schema
    pub fn ipadic() -> Self {
        DictionarySchema {
            name: "IPADIC".to_string(),
            version: "2.7.0".to_string(),
            fields: vec![
                FieldDefinition {
                    index: 0,
                    name: "surface".to_string(),
                    field_type: FieldType::Surface,
                    description: Some("表層形".to_string()),
                },
                FieldDefinition {
                    index: 1,
                    name: "left_context_id".to_string(),
                    field_type: FieldType::LeftContextId,
                    description: Some("左文脈ID".to_string()),
                },
                FieldDefinition {
                    index: 2,
                    name: "right_context_id".to_string(),
                    field_type: FieldType::RightContextId,
                    description: Some("右文脈ID".to_string()),
                },
                FieldDefinition {
                    index: 3,
                    name: "cost".to_string(),
                    field_type: FieldType::Cost,
                    description: Some("コスト".to_string()),
                },
                FieldDefinition {
                    index: 4,
                    name: "major_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("品詞".to_string()),
                },
                FieldDefinition {
                    index: 5,
                    name: "middle_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("品詞細分類1".to_string()),
                },
                FieldDefinition {
                    index: 6,
                    name: "small_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("品詞細分類2".to_string()),
                },
                FieldDefinition {
                    index: 7,
                    name: "fine_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("品詞細分類3".to_string()),
                },
                FieldDefinition {
                    index: 8,
                    name: "conjugation_type".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("活用形".to_string()),
                },
                FieldDefinition {
                    index: 9,
                    name: "conjugation_form".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("活用型".to_string()),
                },
                FieldDefinition {
                    index: 10,
                    name: "base_form".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("原形".to_string()),
                },
                FieldDefinition {
                    index: 11,
                    name: "reading".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("読み".to_string()),
                },
                FieldDefinition {
                    index: 12,
                    name: "pronunciation".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("発音".to_string()),
                },
            ],
        }
    }

    /// UniDic dictionary schema (21 fields)
    pub fn unidic() -> Self {
        DictionarySchema {
            name: "UniDic".to_string(),
            version: "2.1.2".to_string(),
            fields: vec![
                FieldDefinition {
                    index: 0,
                    name: "surface".to_string(),
                    field_type: FieldType::Surface,
                    description: Some("表層形".to_string()),
                },
                FieldDefinition {
                    index: 1,
                    name: "left_context_id".to_string(),
                    field_type: FieldType::LeftContextId,
                    description: Some("左文脈ID".to_string()),
                },
                FieldDefinition {
                    index: 2,
                    name: "right_context_id".to_string(),
                    field_type: FieldType::RightContextId,
                    description: Some("右文脈ID".to_string()),
                },
                FieldDefinition {
                    index: 3,
                    name: "cost".to_string(),
                    field_type: FieldType::Cost,
                    description: Some("コスト".to_string()),
                },
                FieldDefinition {
                    index: 4,
                    name: "major_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("品詞大分類".to_string()),
                },
                FieldDefinition {
                    index: 5,
                    name: "middle_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("品詞中分類".to_string()),
                },
                FieldDefinition {
                    index: 6,
                    name: "small_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("品詞小分類".to_string()),
                },
                FieldDefinition {
                    index: 7,
                    name: "fine_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("品詞細分類".to_string()),
                },
                FieldDefinition {
                    index: 8,
                    name: "conjugation_form".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("活用型".to_string()),
                },
                FieldDefinition {
                    index: 9,
                    name: "conjugation_type".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("活用形".to_string()),
                },
                FieldDefinition {
                    index: 10,
                    name: "lexeme_reading".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("語彙素読み".to_string()),
                },
                FieldDefinition {
                    index: 11,
                    name: "lexeme".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("語彙素（語彙素表記 + 語彙素細分類）".to_string()),
                },
                FieldDefinition {
                    index: 12,
                    name: "orthography_appearance".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("書字形出現形".to_string()),
                },
                FieldDefinition {
                    index: 13,
                    name: "pronunciation_appearance".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("発音形出現形".to_string()),
                },
                FieldDefinition {
                    index: 14,
                    name: "orthography_basic".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("書字形基本形".to_string()),
                },
                FieldDefinition {
                    index: 15,
                    name: "pronunciation_basic".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("発音形基本形".to_string()),
                },
                FieldDefinition {
                    index: 16,
                    name: "word_type".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("語種".to_string()),
                },
                FieldDefinition {
                    index: 17,
                    name: "prefix_form".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("語頭変化型".to_string()),
                },
                FieldDefinition {
                    index: 18,
                    name: "prefix_type".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("語頭変化形".to_string()),
                },
                FieldDefinition {
                    index: 19,
                    name: "suffix_form".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("語末変化型".to_string()),
                },
                FieldDefinition {
                    index: 20,
                    name: "suffix_type".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("語末変化形".to_string()),
                },
            ],
        }
    }

    /// CC-CEDICT dictionary schema
    pub fn cc_cedict() -> Self {
        DictionarySchema {
            name: "CC-CEDICT".to_string(),
            version: "1.0.0".to_string(),
            fields: vec![
                FieldDefinition {
                    index: 0,
                    name: "surface".to_string(),
                    field_type: FieldType::Surface,
                    description: Some("表面形式".to_string()),
                },
                FieldDefinition {
                    index: 1,
                    name: "left_context_id".to_string(),
                    field_type: FieldType::LeftContextId,
                    description: Some("左语境ID".to_string()),
                },
                FieldDefinition {
                    index: 2,
                    name: "right_context_id".to_string(),
                    field_type: FieldType::RightContextId,
                    description: Some("右语境ID".to_string()),
                },
                FieldDefinition {
                    index: 3,
                    name: "cost".to_string(),
                    field_type: FieldType::Cost,
                    description: Some("成本".to_string()),
                },
                FieldDefinition {
                    index: 4,
                    name: "major_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("词类".to_string()),
                },
                FieldDefinition {
                    index: 5,
                    name: "middle_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("词类1".to_string()),
                },
                FieldDefinition {
                    index: 6,
                    name: "small_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("词类2".to_string()),
                },
                FieldDefinition {
                    index: 7,
                    name: "fine_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("词类3".to_string()),
                },
                FieldDefinition {
                    index: 8,
                    name: "pinyin".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("併音".to_string()),
                },
                FieldDefinition {
                    index: 9,
                    name: "traditional".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("繁体字".to_string()),
                },
                FieldDefinition {
                    index: 10,
                    name: "simplified".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("簡体字".to_string()),
                },
                FieldDefinition {
                    index: 11,
                    name: "definition".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("定义".to_string()),
                },
            ],
        }
    }

    /// KO-DIC dictionary schema
    pub fn ko_dic() -> Self {
        DictionarySchema {
            name: "KO-DIC".to_string(),
            version: "1.0.0".to_string(),
            fields: vec![
                FieldDefinition {
                    index: 0,
                    name: "surface".to_string(),
                    field_type: FieldType::Surface,
                    description: Some("표면".to_string()),
                },
                FieldDefinition {
                    index: 1,
                    name: "left_context_id".to_string(),
                    field_type: FieldType::LeftContextId,
                    description: Some("왼쪽 문맥 ID".to_string()),
                },
                FieldDefinition {
                    index: 2,
                    name: "right_context_id".to_string(),
                    field_type: FieldType::RightContextId,
                    description: Some("오른쪽 문맥 ID".to_string()),
                },
                FieldDefinition {
                    index: 3,
                    name: "cost".to_string(),
                    field_type: FieldType::Cost,
                    description: Some("비용".to_string()),
                },
                FieldDefinition {
                    index: 4,
                    name: "pos_tag".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("품사 태그".to_string()),
                },
                FieldDefinition {
                    index: 5,
                    name: "meaning".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("의미 부류".to_string()),
                },
                FieldDefinition {
                    index: 6,
                    name: "presence_absence".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("종성 유무".to_string()),
                },
                FieldDefinition {
                    index: 7,
                    name: "reading".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("읽기".to_string()),
                },
                FieldDefinition {
                    index: 8,
                    name: "type".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("타입".to_string()),
                },
                FieldDefinition {
                    index: 9,
                    name: "first_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("첫번째 품사".to_string()),
                },
                FieldDefinition {
                    index: 10,
                    name: "last_pos".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("마지막 품사".to_string()),
                },
                FieldDefinition {
                    index: 11,
                    name: "expression".to_string(),
                    field_type: FieldType::Custom,
                    description: Some("표현".to_string()),
                },
            ],
        }
    }

    /// Find field by name
    pub fn get_field_by_name(&self, name: &str) -> Option<&FieldDefinition> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get common field index by type
    pub fn get_common_field_index(&self, field_type: &FieldType) -> Option<usize> {
        self.fields
            .iter()
            .find(|f| f.field_type == *field_type)
            .map(|f| f.index)
    }

    /// Get detail fields (index >= 4)
    pub fn get_detail_fields(&self) -> Vec<&FieldDefinition> {
        let mut fields: Vec<&FieldDefinition> =
            self.fields.iter().filter(|f| f.index >= 4).collect();
        fields.sort_by_key(|f| f.index);
        fields
    }

    /// Validate common fields (first 4 columns)
    pub fn validate_common_fields(&self, row: &StringRecord) -> LinderaResult<()> {
        let common_fields = [
            FieldType::Surface,
            FieldType::LeftContextId,
            FieldType::RightContextId,
            FieldType::Cost,
        ];

        for field_type in &common_fields {
            if let Some(index) = self.get_common_field_index(field_type) {
                if index >= row.len() || row[index].trim().is_empty() {
                    return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                        "Common field {:?} is missing or empty",
                        field_type
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipadic_schema() {
        let schema = DictionarySchema::ipadic();
        assert_eq!(schema.name, "IPADIC");
        assert_eq!(schema.version, "2.7.0");
        assert_eq!(schema.fields.len(), 13);

        // Check surface field
        let surface_field = schema.get_field_by_name("surface").unwrap();
        assert_eq!(surface_field.index, 0);
        assert_eq!(surface_field.field_type, FieldType::Surface);

        // Check detail fields
        let detail_fields = schema.get_detail_fields();
        assert_eq!(detail_fields.len(), 9); // 13 - 4 = 9
    }

    #[test]
    fn test_unidic_schema() {
        let schema = DictionarySchema::unidic();
        assert_eq!(schema.name, "UniDic");
        assert_eq!(schema.version, "2.1.2");
        assert_eq!(schema.fields.len(), 21);

        // Check lexeme field
        let lexeme_field = schema.get_field_by_name("lexeme").unwrap();
        assert_eq!(lexeme_field.index, 11);
        assert_eq!(lexeme_field.field_type, FieldType::Custom);

        // Check detail fields
        let detail_fields = schema.get_detail_fields();
        assert_eq!(detail_fields.len(), 17); // 21 - 4 = 17
    }

    #[test]
    fn test_cc_cedict_schema() {
        let schema = DictionarySchema::cc_cedict();
        assert_eq!(schema.name, "CC-CEDICT");
        assert_eq!(schema.version, "1.0.0");
        assert_eq!(schema.fields.len(), 12);

        // Check pinyin field
        let pinyin_field = schema.get_field_by_name("pinyin").unwrap();
        assert_eq!(pinyin_field.index, 8);
        assert_eq!(pinyin_field.field_type, FieldType::Custom);
    }

    #[test]
    fn test_ko_dic_schema() {
        let schema = DictionarySchema::ko_dic();
        assert_eq!(schema.name, "KO-DIC");
        assert_eq!(schema.version, "1.0.0");
        assert_eq!(schema.fields.len(), 12);

        // Check pos_tag field
        let pos_tag_field = schema.get_field_by_name("pos_tag").unwrap();
        assert_eq!(pos_tag_field.index, 4);
        assert_eq!(pos_tag_field.field_type, FieldType::Custom);
    }

    #[test]
    fn test_common_field_index() {
        let schema = DictionarySchema::ipadic();

        assert_eq!(schema.get_common_field_index(&FieldType::Surface), Some(0));
        assert_eq!(
            schema.get_common_field_index(&FieldType::LeftContextId),
            Some(1)
        );
        assert_eq!(
            schema.get_common_field_index(&FieldType::RightContextId),
            Some(2)
        );
        assert_eq!(schema.get_common_field_index(&FieldType::Cost), Some(3));
    }

    #[test]
    fn test_get_field_by_name() {
        let schema = DictionarySchema::ipadic();

        // Valid field name
        let field = schema.get_field_by_name("surface").unwrap();
        assert_eq!(field.index, 0);
        assert_eq!(field.name, "surface");
        assert_eq!(field.field_type, FieldType::Surface);

        // Invalid field name
        assert!(schema.get_field_by_name("nonexistent").is_none());

        // Check all fields exist
        let expected_fields = vec![
            "surface",
            "left_context_id",
            "right_context_id",
            "cost",
            "major_pos",
            "middle_pos",
            "small_pos",
            "fine_pos",
            "conjugation_type",
            "conjugation_form",
            "base_form",
            "reading",
            "pronunciation",
        ];
        for field_name in expected_fields {
            assert!(schema.get_field_by_name(field_name).is_some());
        }
    }

    #[test]
    fn test_get_detail_fields() {
        let schema = DictionarySchema::ipadic();
        let detail_fields = schema.get_detail_fields();

        assert_eq!(detail_fields.len(), 9); // 13 - 4 = 9

        // Check that all detail fields have index >= 4
        for field in &detail_fields {
            assert!(field.index >= 4);
        }

        // Check that fields are sorted by index
        let mut prev_index = 0;
        for field in &detail_fields {
            assert!(field.index > prev_index);
            prev_index = field.index;
        }
    }

    #[test]
    fn test_validate_common_fields_success() {
        let schema = DictionarySchema::ipadic();
        let record = StringRecord::from(vec![
            "surface_form",       // Surface (index 0)
            "123",                // LeftContextId (index 1)
            "456",                // RightContextId (index 2)
            "789",                // Cost (index 3)
            "名詞",               // MajorPos (index 4)
            "一般",               // MiddlePos (index 5)
            "*",                  // SmallPos (index 6)
            "*",                  // FinePos (index 7)
            "*",                  // ConjugationType (index 8)
            "*",                  // ConjugationForm (index 9)
            "surface_form",       // BaseForm (index 10)
            "サーフェスフォーム", // Reading (index 11)
            "サーフェスフォーム", // Pronunciation (index 12)
        ]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_common_fields_empty_field() {
        let schema = DictionarySchema::ipadic();
        let record = StringRecord::from(vec![
            "", // Empty surface (should fail)
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
            "サーフェスフォーム",
            "サーフェスフォーム",
        ]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_common_fields_missing_field() {
        let schema = DictionarySchema::ipadic();
        let record = StringRecord::from(vec![
            "surface_form", // Only first field
        ]);

        let result = schema.validate_common_fields(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_field_type_equality() {
        assert_eq!(FieldType::Surface, FieldType::Surface);
        assert_eq!(FieldType::LeftContextId, FieldType::LeftContextId);
        assert_eq!(FieldType::RightContextId, FieldType::RightContextId);
        assert_eq!(FieldType::Cost, FieldType::Cost);
        assert_eq!(FieldType::Custom, FieldType::Custom);

        assert_ne!(FieldType::Surface, FieldType::Custom);
        assert_ne!(FieldType::LeftContextId, FieldType::RightContextId);
    }

    #[test]
    fn test_dictionary_schema_serialization() {
        let schema = DictionarySchema::ipadic();

        // Test serialization
        let serialized = serde_json::to_string(&schema).unwrap();
        assert!(serialized.contains("IPADIC"));
        assert!(serialized.contains("2.7.0"));

        // Test deserialization
        let deserialized: DictionarySchema = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, schema.name);
        assert_eq!(deserialized.version, schema.version);
        assert_eq!(deserialized.fields.len(), schema.fields.len());
    }

    #[test]
    fn test_field_definition_serialization() {
        let field = FieldDefinition {
            index: 0,
            name: "surface".to_string(),
            field_type: FieldType::Surface,
            description: Some("表層形".to_string()),
        };

        // Test serialization
        let serialized = serde_json::to_string(&field).unwrap();
        assert!(serialized.contains("surface"));
        assert!(serialized.contains("表層形"));

        // Test deserialization
        let deserialized: FieldDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.index, field.index);
        assert_eq!(deserialized.name, field.name);
        assert_eq!(deserialized.field_type, field.field_type);
        assert_eq!(deserialized.description, field.description);
    }

    #[test]
    fn test_all_dictionary_schemas_have_common_fields() {
        let schemas = vec![
            DictionarySchema::ipadic(),
            DictionarySchema::unidic(),
            DictionarySchema::cc_cedict(),
            DictionarySchema::ko_dic(),
        ];

        for schema in schemas {
            // Check that all schemas have the 4 common fields
            assert!(schema.get_common_field_index(&FieldType::Surface).is_some());
            assert!(
                schema
                    .get_common_field_index(&FieldType::LeftContextId)
                    .is_some()
            );
            assert!(
                schema
                    .get_common_field_index(&FieldType::RightContextId)
                    .is_some()
            );
            assert!(schema.get_common_field_index(&FieldType::Cost).is_some());

            // Check that common fields are at expected indices
            assert_eq!(schema.get_common_field_index(&FieldType::Surface), Some(0));
            assert_eq!(
                schema.get_common_field_index(&FieldType::LeftContextId),
                Some(1)
            );
            assert_eq!(
                schema.get_common_field_index(&FieldType::RightContextId),
                Some(2)
            );
            assert_eq!(schema.get_common_field_index(&FieldType::Cost), Some(3));
        }
    }

    #[test]
    fn test_unidic_specific_fields() {
        let schema = DictionarySchema::unidic();

        // Check specific UniDic fields
        assert!(schema.get_field_by_name("lexeme_reading").is_some());
        assert!(schema.get_field_by_name("lexeme").is_some());
        assert!(schema.get_field_by_name("orthography_appearance").is_some());
        assert!(
            schema
                .get_field_by_name("pronunciation_appearance")
                .is_some()
        );
        assert!(schema.get_field_by_name("word_type").is_some());
        assert!(schema.get_field_by_name("prefix_form").is_some());
        assert!(schema.get_field_by_name("suffix_type").is_some());
    }

    #[test]
    fn test_cc_cedict_specific_fields() {
        let schema = DictionarySchema::cc_cedict();

        // Check specific CC-CEDICT fields
        assert!(schema.get_field_by_name("pinyin").is_some());
        assert!(schema.get_field_by_name("traditional").is_some());
        assert!(schema.get_field_by_name("simplified").is_some());
        assert!(schema.get_field_by_name("definition").is_some());

        // Check field indices
        assert_eq!(schema.get_field_by_name("pinyin").unwrap().index, 8);
        assert_eq!(schema.get_field_by_name("traditional").unwrap().index, 9);
        assert_eq!(schema.get_field_by_name("simplified").unwrap().index, 10);
        assert_eq!(schema.get_field_by_name("definition").unwrap().index, 11);
    }

    #[test]
    fn test_ko_dic_specific_fields() {
        let schema = DictionarySchema::ko_dic();

        // Check specific KO-DIC fields
        assert!(schema.get_field_by_name("pos_tag").is_some());
        assert!(schema.get_field_by_name("meaning").is_some());
        assert!(schema.get_field_by_name("presence_absence").is_some());
        assert!(schema.get_field_by_name("reading").is_some());
        assert!(schema.get_field_by_name("type").is_some());
        assert!(schema.get_field_by_name("first_pos").is_some());
        assert!(schema.get_field_by_name("last_pos").is_some());
        assert!(schema.get_field_by_name("expression").is_some());
    }
}
