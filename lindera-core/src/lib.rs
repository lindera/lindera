pub mod assets;
pub mod compress;
pub mod decompress;
pub mod dictionary;
pub mod dictionary_builder;
pub mod error;
pub mod mode;

pub type LinderaResult<T> = Result<T, crate::error::LinderaError>;
