#[cfg(feature = "embed-ipadic-neologd")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "ipadic-neologd";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
