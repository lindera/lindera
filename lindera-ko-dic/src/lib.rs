#[cfg(feature = "ko-dic")]
pub mod metadata;
#[cfg(feature = "ko-dic")]
pub mod schema;

#[cfg(feature = "embedded-ko-dic")]
pub mod embedded;

#[cfg(feature = "ko-dic")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(all(feature = "ko-dic", not(feature = "embedded-ko-dic")))]
use lindera_dictionary::dictionary_loader::StandardDictionaryLoader;
#[cfg(feature = "ko-dic")]
use metadata::KoDicMetadata;

#[cfg(feature = "ko-dic")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(KoDicMetadata::default())
}

#[cfg(all(feature = "ko-dic", not(feature = "embedded-ko-dic")))]
pub fn create_loader() -> DictionaryLoader {
    DictionaryLoader::new(
        "Ko-Dic".to_string(),
        vec![
            "./dict/ko-dic".to_string(),
            "./lindera-ko-dic".to_string(),
            "/usr/local/share/lindera/ko-dic".to_string(),
            "/usr/share/lindera/ko-dic".to_string(),
        ],
        "LINDERA_KO_DIC_PATH".to_string(),
    )
}

#[cfg(feature = "embedded-ko-dic")]
pub fn create_loader() -> EmbeddedLoader {
    EmbeddedLoader
}

#[cfg(feature = "embedded-ko-dic")]
pub struct EmbeddedLoader;

#[cfg(feature = "embedded-ko-dic")]
impl EmbeddedLoader {
    pub fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

#[cfg(feature = "embedded-ko-dic")]
impl DictionaryLoader for EmbeddedLoader {
    fn load(&self) -> LinderaResult<lindera_dictionary::dictionary::Dictionary> {
        embedded::load()
    }
}

#[cfg(feature = "embedded-ko-dic")]
use lindera_dictionary::LinderaResult;
#[cfg(feature = "embedded-ko-dic")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
