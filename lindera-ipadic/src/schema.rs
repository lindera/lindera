use lindera_dictionary::dictionary::schema::Schema;

/// IPADIC dictionary schema factory
pub struct IpadicSchema;

impl Default for IpadicSchema {
    fn default() -> Self {
        Self
    }
}

impl IpadicSchema {
    /// Create default IPADIC schema
    pub fn schema() -> Schema {
        Schema::new(
            "IPADIC".to_string(),
            "2.7.0-20070801".to_string(),
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
}
