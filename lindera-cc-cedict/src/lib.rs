#[cfg(feature = "embedded-cc-cedict")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "cc-cedict";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
