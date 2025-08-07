use lindera_dictionary::dictionary::schema::Schema;

pub struct KoDicSchema;

impl KoDicSchema {
    pub fn default() -> Schema {
        Schema::ko_dic()
    }
}
