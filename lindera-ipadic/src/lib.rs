#[cfg(feature = "embedded-ipadic")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "ipadic";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
