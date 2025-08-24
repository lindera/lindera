#[cfg(feature = "embedded-ipadic-neologd")]
pub mod metadata;
#[cfg(feature = "embedded-ipadic-neologd")]
pub mod schema;

#[cfg(feature = "embedded-ipadic-neologd")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "ipadic-neologd";
pub const DICTIONARY_ENCODING: &str = "UTF-8";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
