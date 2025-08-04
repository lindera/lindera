use lindera_dictionary::dictionary_builder::Schema;

pub struct KoDicSchema;

impl KoDicSchema {
    pub fn new() -> Schema {
        Schema::ko_dic()
    }
}

impl Default for KoDicSchema {
    fn default() -> Self {
        Self
    }
}

impl From<KoDicSchema> for Schema {
    fn from(_: KoDicSchema) -> Self {
        Schema::ko_dic()
    }
}