pub mod analyzer;
pub mod builder;
pub mod character_filter;
pub mod error;
pub mod mode;
pub mod token_filter;
pub mod tokenizer;

use std::str::FromStr;

use error::LinderaErrorKind;
use serde::{Deserialize, Serialize};

use crate::error::LinderaError;
use lindera_core::token::Token as LinderaToken;

pub type LinderaResult<T> = Result<T, LinderaError>;
pub type Token<'a> = LinderaToken<'a>;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryKind {
    #[serde(rename = "ipadic")]
    IPADIC,
    #[serde(rename = "unidic")]
    UniDic,
    #[serde(rename = "ko-dic")]
    KoDic,
    #[serde(rename = "cc-cedict")]
    CcCedict,
}

impl FromStr for DictionaryKind {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryKind, Self::Err> {
        match input {
            "ipadic" => Ok(DictionaryKind::IPADIC),
            "unidic" => Ok(DictionaryKind::UniDic),
            "ko-dic" => Ok(DictionaryKind::KoDic),
            "cc-cedict" => Ok(DictionaryKind::CcCedict),
            _ => Err(LinderaErrorKind::DictionaryKindError
                .with_error(anyhow::anyhow!("Invalid dictionary kind: {}", input))),
        }
    }
}
