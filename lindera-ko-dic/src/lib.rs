pub mod loader;

#[cfg(feature = "ko-dic")]
pub mod builder;
#[cfg(feature = "ko-dic")]
pub mod metadata;
#[cfg(feature = "ko-dic")]
pub mod schema;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
