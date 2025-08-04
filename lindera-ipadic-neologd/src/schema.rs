use lindera_dictionary::dictionary_builder::Schema;

pub struct IpadicNeologdSchema;

impl IpadicNeologdSchema {
    pub fn new() -> Schema {
        Schema::ipadic_neologd()
    }
}

impl Default for IpadicNeologdSchema {
    fn default() -> Self {
        Self
    }
}

impl From<IpadicNeologdSchema> for Schema {
    fn from(_: IpadicNeologdSchema) -> Self {
        Schema::ipadic_neologd()
    }
}