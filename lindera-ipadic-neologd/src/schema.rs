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
        Schema::new(
            "IPADIC-NEologd".to_string(),
            "0.0.7-20200820".to_string(),
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
