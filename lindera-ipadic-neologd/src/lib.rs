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

#[cfg(feature = "ipadic-neologd")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(all(feature = "ipadic-neologd", not(feature = "embedded-ipadic-neologd")))]
use lindera_dictionary::dictionary_loader::StandardDictionaryLoader;
#[cfg(feature = "ipadic-neologd")]
use metadata::IpadicNeologdMetadata;

#[cfg(feature = "ipadic-neologd")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(IpadicNeologdMetadata::metadata())
}

#[cfg(all(feature = "ipadic-neologd", not(feature = "embedded-ipadic-neologd")))]
pub fn create_loader() -> StandardDictionaryLoader {
    StandardDictionaryLoader::new(
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

#[cfg(feature = "embedded-ipadic-neologd")]
pub fn create_loader() -> EmbeddedLoader {
    EmbeddedLoader
}

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
