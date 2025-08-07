use lindera_dictionary::dictionary::schema::Schema;

pub struct CcCedictSchema;

impl CcCedictSchema {
    pub fn default() -> Schema {
        Schema::cc_cedict()
    }
}
