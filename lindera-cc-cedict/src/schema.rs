use lindera_dictionary::dictionary::schema::Schema;

pub struct CcCedictSchema;

impl Default for CcCedictSchema {
    fn default() -> Self {
        Self
    }
}

impl CcCedictSchema {
    pub fn schema() -> Schema {
        Schema::new(vec![
            "major_pos".to_string(),
            "middle_pos".to_string(),
            "small_pos".to_string(),
            "fine_pos".to_string(),
            "pinyin".to_string(),
            "traditional".to_string(),
            "simplified".to_string(),
            "definition".to_string(),
        ])
    }
}
