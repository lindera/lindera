pub mod character_definition;
pub mod connection;
pub mod error;
pub mod prefix_dict;
pub mod unknown_dictionary;
pub mod viterbi;
pub mod word_entry;

use crate::error::LinderaError;

pub type LinderaResult<T> = Result<T, LinderaError>;
