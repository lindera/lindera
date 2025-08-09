#[cfg(feature = "ipadic-neologd")]
pub mod metadata;
#[cfg(feature = "ipadic-neologd")]
pub mod schema;

#[cfg(feature = "embedded-ipadic-neologd")]
pub mod embedded;

#[cfg(feature = "embedded-ipadic-neologd")]
use lindera_dictionary::LinderaResult;
#[cfg(feature = "embedded-ipadic-neologd")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;

#[cfg(feature = "embedded-ipadic-neologd")]
pub struct EmbeddedLoader;

#[cfg(feature = "embedded-ipadic-neologd")]
impl EmbeddedLoader {
    pub fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

#[cfg(feature = "embedded-ipadic-neologd")]
impl DictionaryLoader for EmbeddedLoader {
    fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
