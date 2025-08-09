#[cfg(feature = "ipadic")]
pub mod metadata;
#[cfg(feature = "ipadic")]
pub mod schema;

#[cfg(feature = "embedded-ipadic")]
pub mod embedded;

#[cfg(feature = "ipadic")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(all(feature = "ipadic", not(feature = "embedded-ipadic")))]
use lindera_dictionary::dictionary_loader::StandardDictionaryLoader;
#[cfg(feature = "ipadic")]
use metadata::IpadicMetadata;

#[cfg(feature = "ipadic")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(IpadicMetadata::default())
}

#[cfg(all(feature = "ipadic", not(feature = "embedded-ipadic")))]
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

#[cfg(feature = "embedded-ipadic")]
pub fn create_loader() -> EmbeddedLoader {
    EmbeddedLoader
}

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

#[cfg(feature = "embedded-ipadic")]
use lindera_dictionary::LinderaResult;
#[cfg(feature = "embedded-ipadic")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
