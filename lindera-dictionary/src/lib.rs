#[cfg(feature = "build_rs")]
pub mod assets;
pub mod builder;
pub mod compress;
pub mod decompress;
pub mod dictionary;
pub mod error;
pub mod loader;
pub mod macros;
pub mod mode;
pub mod nbest;
pub mod util;
pub mod viterbi;

#[cfg(feature = "train")]
pub mod trainer;

pub type LinderaResult<T> = Result<T, crate::error::LinderaError>;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
