#[cfg(feature = "ko-dic")]
pub mod metadata;
#[cfg(feature = "ko-dic")]
pub mod schema;

#[cfg(feature = "embedded-ko-dic")]
pub mod embedded;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
