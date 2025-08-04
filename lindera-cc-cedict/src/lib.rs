pub mod cc_cedict;

#[cfg(feature = "cc-cedict")]
pub mod builder;
#[cfg(feature = "cc-cedict")]
pub mod metadata;
#[cfg(feature = "cc-cedict")]
pub mod schema;

// Re-export for convenient access
#[cfg(feature = "cc-cedict")]
pub use builder::CcCedictBuilder;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
