#[cfg(feature = "embedded-cc-cedict")]
pub mod metadata;
#[cfg(feature = "embedded-cc-cedict")]
pub mod schema;

#[cfg(feature = "embedded-cc-cedict")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "cc-cedict";
pub const DICTIONARY_ENCODING: &str = "UTF-8";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
