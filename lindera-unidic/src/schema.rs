use lindera_dictionary::dictionary::schema::Schema;

/// UniDic dictionary schema factory
pub struct UniDicSchema;

impl Default for UniDicSchema {
    fn default() -> Self {
        Self
    }
}

impl UniDicSchema {
    /// Create default UniDic schema
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
            "conjugation_type".to_string(),
            "conjugation_form".to_string(),
            "reading".to_string(),
            "lexeme".to_string(),
            "orthographic_surface_form".to_string(),
            "phonological_surface_form".to_string(),
            "orthographic_base_form".to_string(),
            "phonological_base_form".to_string(),
            "word_type".to_string(),
            "initial_mutation_type".to_string(),
            "initial_mutation_form".to_string(),
            "final_mutation_type".to_string(),
            "final_mutation_form".to_string(),
        ])
    }
}

pub struct UniDicUserDictionarySchema;

impl Default for UniDicUserDictionarySchema {
    fn default() -> Self {
        Self
    }
}

impl UniDicUserDictionarySchema {
    pub fn schema() -> Schema {
        Schema::new(vec![
            "surface".to_string(),
            "part_of_speech".to_string(),
            "reading".to_string(),
        ])
    }
}
