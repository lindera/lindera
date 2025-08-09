#[cfg(feature = "ipadic")]
pub mod metadata;
#[cfg(feature = "ipadic")]
pub mod schema;

#[cfg(feature = "embedded-ipadic")]
pub mod embedded;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
