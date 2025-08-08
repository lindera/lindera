#[cfg(feature = "cc-cedict")]
pub mod metadata;
#[cfg(feature = "cc-cedict")]
pub mod schema;

#[cfg(feature = "embedded-cc-cedict")]
pub mod embedded;

#[cfg(feature = "cc-cedict")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(all(feature = "cc-cedict", not(feature = "embedded-cc-cedict")))]
use lindera_dictionary::dictionary_loader::DictionaryLoader;
#[cfg(feature = "cc-cedict")]
use metadata::CcCedictMetadata;

#[cfg(feature = "cc-cedict")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(CcCedictMetadata::default())
}

#[cfg(all(feature = "cc-cedict", not(feature = "embedded-cc-cedict")))]
pub fn create_loader() -> DictionaryLoader {
    DictionaryLoader::new(
        "CC-CEDICT".to_string(),
        vec![
            "./dict/cc-cedict".to_string(),
            "./lindera-cc-cedict".to_string(),
            "/usr/local/share/lindera/cc-cedict".to_string(),
            "/usr/share/lindera/cc-cedict".to_string(),
        ],
        "LINDERA_CC_CEDICT_PATH".to_string(),
    )
}

#[cfg(feature = "embedded-cc-cedict")]
pub fn create_loader() -> EmbeddedLoader {
    EmbeddedLoader
}

#[cfg(feature = "embedded-cc-cedict")]
pub struct EmbeddedLoader;

#[cfg(feature = "embedded-cc-cedict")]
impl EmbeddedLoader {
    pub fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

#[cfg(feature = "embedded-cc-cedict")]
use lindera_dictionary::LinderaResult;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
