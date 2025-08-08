#[cfg(feature = "ipadic")]
pub mod metadata;
#[cfg(feature = "ipadic")]
pub mod schema;

#[cfg(feature = "ipadic")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(feature = "ipadic")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;
#[cfg(feature = "ipadic")]
use metadata::IpadicMetadata;

#[cfg(feature = "ipadic")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(IpadicMetadata::default())
}

#[cfg(feature = "ipadic")]
pub fn create_loader() -> DictionaryLoader {
    DictionaryLoader::new(
        "IPADIC".to_string(),
        vec![
            "./dict/ipadic".to_string(),
            "./lindera-ipadic".to_string(),
            "/usr/local/share/lindera/ipadic".to_string(),
            "/usr/share/lindera/ipadic".to_string(),
        ],
        "LINDERA_IPADIC_PATH".to_string(),
    )
}

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
