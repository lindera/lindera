#[cfg(feature = "ko-dic")]
pub mod metadata;
#[cfg(feature = "ko-dic")]
pub mod schema;

#[cfg(feature = "ko-dic")]
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
#[cfg(feature = "ko-dic")]
use lindera_dictionary::dictionary_loader::DictionaryLoader;
#[cfg(feature = "ko-dic")]
use metadata::KoDicMetadata;

#[cfg(feature = "ko-dic")]
pub fn create_builder() -> DictionaryBuilder {
    DictionaryBuilder::new(KoDicMetadata::default())
}

#[cfg(feature = "ko-dic")]
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

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
