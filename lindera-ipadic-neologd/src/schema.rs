use lindera_dictionary::dictionary::schema::Schema;

pub struct IpadicNeologdSchema;

impl IpadicNeologdSchema {
    pub fn default() -> Schema {
        Schema::ipadic_neologd()
    }
}
