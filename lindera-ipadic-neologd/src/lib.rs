pub mod loader;

#[cfg(feature = "ipadic-neologd")]
pub mod builder;
#[cfg(feature = "ipadic-neologd")]
pub mod metadata;
#[cfg(feature = "ipadic-neologd")]
pub mod schema;


const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
