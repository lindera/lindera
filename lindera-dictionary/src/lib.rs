#[cfg(feature = "build_rs")]
pub mod assets;
pub mod compress;
pub mod decompress;
pub mod dictionary;
pub mod dictionary_builder;
pub mod dictionary_loader;
pub mod error;
pub mod mode;
pub mod util;
pub mod viterbi;

pub type LinderaResult<T> = Result<T, crate::error::LinderaError>;
