#[cfg(feature = "ipadic")]
pub mod metadata;
#[cfg(feature = "ipadic")]
pub mod schema;

#[cfg(feature = "embedded-ipadic")]
pub mod embedded;

#[cfg(feature = "embedded-ipadic")]
use lindera_dictionary::LinderaResult;
#[cfg(feature = "embedded-ipadic")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;

#[cfg(feature = "embedded-ipadic")]
pub struct EmbeddedLoader;

#[cfg(feature = "embedded-ipadic")]
impl EmbeddedLoader {
    pub fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

#[cfg(feature = "embedded-ipadic")]
impl DictionaryLoader for EmbeddedLoader {
    fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
