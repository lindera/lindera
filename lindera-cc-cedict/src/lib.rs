#[cfg(feature = "embed-cc-cedict")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "cc-cedict";
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERSION
}
