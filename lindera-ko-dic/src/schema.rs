use lindera_dictionary::dictionary::schema::Schema;

pub struct KoDicSchema;

impl Default for KoDicSchema {
    fn default() -> Self {
        Self
    }
}

impl KoDicSchema {
    pub fn schema() -> Schema {
        Schema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "part_of_speech_tag".to_string(),
            "meaning".to_string(),
            "presence_absence".to_string(),
            "reading".to_string(),
            "type".to_string(),
            "first_part_of_speech".to_string(),
            "last_part_of_speech".to_string(),
            "expression".to_string(),
        ])
    }
}

pub struct KoDicUserDictionarySchema;

impl Default for KoDicUserDictionarySchema {
    fn default() -> Self {
        Self
    }
}

impl KoDicUserDictionarySchema {
    pub fn schema() -> Schema {
        Schema::new(vec![
            "surface".to_string(),
            "part_of_speech_tag".to_string(),
            "reading".to_string(),
        ])
    }
}
