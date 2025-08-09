use lindera_dictionary::dictionary::schema::Schema;

pub struct KoDicSchema;

impl Default for KoDicSchema {
    fn default() -> Self {
        Self
    }
}

impl KoDicSchema {
    pub fn schema() -> Schema {
        Schema::new(
            "KO-DIC".to_string(),
            "2.1.1-20180720".to_string(),
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
