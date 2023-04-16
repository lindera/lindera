pub mod analyzer;
pub mod character_filter;
pub mod error;
pub mod mode;
pub mod token_filter;

use crate::error::LinderaError;

pub type LinderaResult<T> = Result<T, LinderaError>;
pub type Token<'a> = lindera_tokenizer::token::Token<'a>;
pub type DictionaryKind = lindera_dictionary::DictionaryKind;
pub type FilteredToken = lindera_filter::token::FilteredToken;
pub type Dictionary = lindera_core::dictionary::Dictionary;
pub type UserDictionary = lindera_core::user_dictionary::UserDictionary;
