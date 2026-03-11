#[cfg(feature = "embed-jieba")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "jieba";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
