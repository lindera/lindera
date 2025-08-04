use lindera_dictionary::dictionary_builder::Schema;

pub struct CcCedictSchema;

impl CcCedictSchema {
    pub fn new() -> Schema {
        Schema::cc_cedict()
    }
}

impl Default for CcCedictSchema {
    fn default() -> Self {
        Self
    }
}

impl From<CcCedictSchema> for Schema {
    fn from(_: CcCedictSchema) -> Self {
        Schema::cc_cedict()
    }
}