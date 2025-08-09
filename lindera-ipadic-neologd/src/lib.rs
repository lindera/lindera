#[cfg(feature = "ipadic-neologd")]
pub mod metadata;
#[cfg(feature = "ipadic-neologd")]
pub mod schema;

#[cfg(feature = "embedded-ipadic-neologd")]
pub mod embedded;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
