#[cfg(feature = "cc-cedict")]
pub mod metadata;
#[cfg(feature = "cc-cedict")]
pub mod schema;

#[cfg(feature = "embedded-cc-cedict")]
pub mod embedded;

#[cfg(feature = "embedded-cc-cedict")]
use lindera_dictionary::LinderaResult;
#[cfg(feature = "embedded-cc-cedict")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;

#[cfg(feature = "embedded-cc-cedict")]
pub struct EmbeddedLoader;

#[cfg(feature = "embedded-cc-cedict")]
impl EmbeddedLoader {
    pub fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

#[cfg(feature = "embedded-cc-cedict")]
impl DictionaryLoader for EmbeddedLoader {
    fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
