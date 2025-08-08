use lindera_dictionary::dictionary::schema::Schema;

pub struct KoDicSchema;

impl KoDicSchema {
    pub fn default() -> Schema {
        Schema::new(
            "KO-DIC".to_string(),
            "1.0.0".to_string(),
            vec![
                "pos_tag".to_string(),
                "meaning".to_string(),
                "presence_absence".to_string(),
                "reading".to_string(),
                "type".to_string(),
                "first_pos".to_string(),
                "last_pos".to_string(),
                "expression".to_string(),
            ],
        )
    }
}
