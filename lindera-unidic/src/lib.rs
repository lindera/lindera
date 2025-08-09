#[cfg(feature = "unidic")]
pub mod metadata;
#[cfg(feature = "unidic")]
pub mod schema;

#[cfg(feature = "embedded-unidic")]
pub mod embedded;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
