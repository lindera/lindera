#[cfg(feature = "embedded-unidic")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "unidic";
pub const DICTIONARY_ENCODING: &str = "UTF-8";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
