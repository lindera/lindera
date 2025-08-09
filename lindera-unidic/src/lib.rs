#[cfg(feature = "unidic")]
pub mod metadata;
#[cfg(feature = "unidic")]
pub mod schema;

#[cfg(feature = "embedded-unidic")]
pub mod embedded;

#[cfg(feature = "unidic")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(all(feature = "unidic", not(feature = "embedded-unidic")))]
use lindera_dictionary::dictionary_loader::StandardDictionaryLoader;
#[cfg(feature = "unidic")]
use metadata::UnidicMetadata;

#[cfg(feature = "unidic")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(UnidicMetadata::default())
}

#[cfg(all(feature = "unidic", not(feature = "embedded-unidic")))]
pub fn create_loader() -> DictionaryLoader {
    DictionaryLoader::new(
        "UniDic".to_string(),
        vec![
            "./dict/unidic".to_string(),
            "./lindera-unidic".to_string(),
            "/usr/local/share/lindera/unidic".to_string(),
            "/usr/share/lindera/unidic".to_string(),
        ],
        "LINDERA_UNIDIC_PATH".to_string(),
    )
}

#[cfg(feature = "embedded-unidic")]
pub fn create_loader() -> EmbeddedLoader {
    EmbeddedLoader
}

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

#[cfg(feature = "embedded-unidic")]
use lindera_dictionary::LinderaResult;
#[cfg(feature = "embedded-unidic")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
