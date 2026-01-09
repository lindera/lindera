#[cfg(feature = "embed-ko-dic")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "ko-dic";
const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
