pub mod unidic;

#[cfg(feature = "unidic")]
pub mod builder;
#[cfg(feature = "unidic")]
pub mod metadata;
#[cfg(feature = "unidic")]
pub mod schema;

// Re-export for convenient access
#[cfg(feature = "unidic")]
pub use builder::UnidicBuilder;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
