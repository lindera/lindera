pub mod ko_dic;

#[cfg(feature = "ko-dic")]
pub mod builder;
#[cfg(feature = "ko-dic")]
pub mod metadata;
#[cfg(feature = "ko-dic")]
pub mod schema;

// Re-export for convenient access
#[cfg(feature = "ko-dic")]
pub use builder::KoDicBuilder;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
