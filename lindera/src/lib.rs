#[cfg(any(feature = "ipadic", feature = "unidic", feature = "ko-dic",))]
pub mod analyzer;
pub mod dictionary;
pub mod error;
pub mod mode;
pub mod tokenizer;

use crate::error::LinderaError;

pub type LinderaResult<T> = Result<T, LinderaError>;
pub type Token<'a> = lindera_core::token::Token<'a>;
pub type DictionaryKind = lindera_dictionary::DictionaryKind;
pub type FilteredToken = lindera_filter::token::FilteredToken;
pub type Dictionary = lindera_core::dictionary::Dictionary;
pub type UserDictionary = lindera_core::user_dictionary::UserDictionary;
