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
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "part_of_speech".to_string(),
            "part_of_speech_subcategory_1".to_string(),
            "part_of_speech_subcategory_2".to_string(),
            "part_of_speech_subcategory_3".to_string(),
            "pinyin".to_string(),
            "traditional".to_string(),
            "simplified".to_string(),
            "definition".to_string(),
        ])
    }
}
