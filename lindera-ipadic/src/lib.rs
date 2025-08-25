#[cfg(feature = "embedded-ipadic")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "ipadic";
pub const DICTIONARY_ENCODING: &str = "EUC-JP";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
