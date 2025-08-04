pub mod ipadic_neologd;

#[cfg(feature = "ipadic-neologd")]
pub mod builder;
#[cfg(feature = "ipadic-neologd")]
pub mod metadata;
#[cfg(feature = "ipadic-neologd")]
pub mod schema;

// Re-export for convenient access
#[cfg(feature = "ipadic-neologd")]
pub use builder::IpadicNeologdBuilder;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
