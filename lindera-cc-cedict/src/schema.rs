use lindera_dictionary::dictionary::schema::Schema;

pub struct CcCedictSchema;

impl CcCedictSchema {
    pub fn default() -> Schema {
        Schema::new(
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
}
