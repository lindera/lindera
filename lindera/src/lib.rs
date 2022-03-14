pub mod error;
pub mod formatter;
pub mod mode;
pub mod tokenizer;

use crate::error::LinderaError;

pub type LinderaResult<T> = Result<T, LinderaError>;
