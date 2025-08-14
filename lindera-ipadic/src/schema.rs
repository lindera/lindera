use lindera_dictionary::dictionary::schema::Schema;

/// IPADIC dictionary schema factory
pub struct IPADICSchema;

impl Default for IPADICSchema {
    fn default() -> Self {
        Self
    }
}

impl IPADICSchema {
    /// Create default IPADIC schema
    pub fn schema() -> Schema {
        Schema::new(vec![
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
