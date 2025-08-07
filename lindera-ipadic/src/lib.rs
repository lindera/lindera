pub mod loader;

#[cfg(feature = "ipadic")]
pub mod builder;
#[cfg(feature = "ipadic")]
pub mod metadata;
#[cfg(feature = "ipadic")]
pub mod schema;


const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
