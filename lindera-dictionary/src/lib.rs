#[cfg(feature = "build_rs")]
pub mod assets;
pub mod compress;
pub mod decompress;
pub mod dictionary;
pub mod dictionary_builder;
pub mod dictionary_loader;
pub mod error;
pub mod macros;
pub mod mode;
pub mod util;
pub mod viterbi;

#[cfg(feature = "train")]
pub mod trainer;

pub type LinderaResult<T> = Result<T, crate::error::LinderaError>;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}
