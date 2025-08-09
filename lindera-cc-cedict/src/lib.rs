#[cfg(feature = "cc-cedict")]
pub mod metadata;
#[cfg(feature = "cc-cedict")]
pub mod schema;

#[cfg(feature = "embedded-cc-cedict")]
pub mod embedded;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
