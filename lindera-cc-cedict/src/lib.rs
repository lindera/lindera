#[cfg(feature = "cc-cedict")]
pub mod metadata;
#[cfg(feature = "cc-cedict")]
pub mod schema;

#[cfg(feature = "cc-cedict")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(feature = "cc-cedict")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;
#[cfg(feature = "cc-cedict")]
use metadata::CcCedictMetadata;

#[cfg(feature = "cc-cedict")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(CcCedictMetadata::default())
}

#[cfg(feature = "cc-cedict")]
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

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
