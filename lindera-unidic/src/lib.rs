#[cfg(feature = "unidic")]
pub mod metadata;
#[cfg(feature = "unidic")]
pub mod schema;

#[cfg(feature = "unidic")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(feature = "unidic")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;
#[cfg(feature = "unidic")]
use metadata::UnidicMetadata;

#[cfg(feature = "unidic")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(UnidicMetadata::default())
}

#[cfg(feature = "unidic")]
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

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
