#[cfg(feature = "ipadic-neologd")]
pub mod metadata;
#[cfg(feature = "ipadic-neologd")]
pub mod schema;

#[cfg(feature = "ipadic-neologd")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(feature = "ipadic-neologd")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;
#[cfg(feature = "ipadic-neologd")]
use metadata::IpadicNeologdMetadata;

#[cfg(feature = "ipadic-neologd")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(IpadicNeologdMetadata::default())
}

#[cfg(feature = "ipadic-neologd")]
pub fn create_loader() -> DictionaryLoader {
    DictionaryLoader::new(
        "IPADIC-NEologd".to_string(),
        vec![
            "./dict/ipadic-neologd".to_string(),
            "./lindera-ipadic-neologd".to_string(),
            "/usr/local/share/lindera/ipadic-neologd".to_string(),
            "/usr/share/lindera/ipadic-neologd".to_string(),
        ],
        "LINDERA_IPADIC_NEOLOGD_PATH".to_string(),
    )
}

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
