pub mod ipadic;

#[cfg(feature = "ipadic")]
pub mod builder;
#[cfg(feature = "ipadic")]
pub mod metadata;
#[cfg(feature = "ipadic")]
pub mod schema;

// Re-export for convenient access
#[cfg(feature = "ipadic")]
pub use builder::IpadicBuilder;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
