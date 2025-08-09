#[cfg(feature = "unidic")]
pub mod metadata;
#[cfg(feature = "unidic")]
pub mod schema;

#[cfg(feature = "embedded-unidic")]
pub mod embedded;

#[cfg(feature = "embedded-unidic")]
use lindera_dictionary::LinderaResult;
#[cfg(feature = "embedded-unidic")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;

#[cfg(feature = "embedded-unidic")]
pub struct EmbeddedLoader;

#[cfg(feature = "embedded-unidic")]
impl EmbeddedLoader {
    pub fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

#[cfg(feature = "embedded-unidic")]
impl DictionaryLoader for EmbeddedLoader {
    fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
