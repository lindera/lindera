pub mod loader;

#[cfg(feature = "cc-cedict")]
pub mod builder;
#[cfg(feature = "cc-cedict")]
pub mod metadata;
#[cfg(feature = "cc-cedict")]
pub mod schema;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
