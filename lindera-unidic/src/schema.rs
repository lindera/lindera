use lindera_dictionary::dictionary::schema::Schema;

/// UniDic dictionary schema factory
pub struct UnidicSchema;

impl UnidicSchema {
    /// Create default UniDic schema
    pub fn default() -> Schema {
        Schema::new(
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
}
