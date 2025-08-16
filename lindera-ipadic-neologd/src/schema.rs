use lindera_dictionary::dictionary::schema::Schema;

/// IPADIC NEologd dictionary schema factory
pub struct IPADICNEologdSchema;

impl Default for IPADICNEologdSchema {
    fn default() -> Self {
        Self
    }
}

impl IPADICNEologdSchema {
    /// Create default IPADIC NEologd schema
    pub fn schema() -> Schema {
        Schema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "part_of_speech".to_string(),
            "part_of_speech_subcategory_1".to_string(),
            "part_of_speech_subcategory_2".to_string(),
            "part_of_speech_subcategory_3".to_string(),
            "conjugation_form".to_string(),
            "conjugation_type".to_string(),
            "base_form".to_string(),
            "reading".to_string(),
            "pronunciation".to_string(),
        ])
    }
}
