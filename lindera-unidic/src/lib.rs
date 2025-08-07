pub mod loader;

#[cfg(feature = "unidic")]
pub mod builder;
#[cfg(feature = "unidic")]
pub mod metadata;
#[cfg(feature = "unidic")]
pub mod schema;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
