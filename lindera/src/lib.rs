pub mod builder;
pub mod error;
pub mod mode;
pub mod tokenizer;

use std::str::FromStr;

use error::LinderaErrorKind;
use serde::{Deserialize, Serialize};

use crate::error::LinderaError;

pub type LinderaResult<T> = Result<T, LinderaError>;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryKind {
    #[cfg(feature = "ipadic")]
    #[serde(rename = "ipadic")]
    IPADIC,
    #[cfg(feature = "unidic")]
    #[serde(rename = "unidic")]
    UniDic,
    #[cfg(feature = "ko-dic")]
    #[serde(rename = "ko-dic")]
    KoDic,
    #[cfg(feature = "cc-cedict")]
    #[serde(rename = "cc-cedict")]
    CcCedict,
}

impl FromStr for DictionaryKind {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryKind, Self::Err> {
        match input {
            #[cfg(feature = "ipadic")]
            "ipadic" => Ok(DictionaryKind::IPADIC),
            #[cfg(feature = "unidic")]
            "unidic" => Ok(DictionaryKind::UniDic),
            #[cfg(feature = "ko-dic")]
            "ko-dic" => Ok(DictionaryKind::KoDic),
            #[cfg(feature = "cc-cedict")]
            "cc-cedict" => Ok(DictionaryKind::CcCedict),
            _ => Err(LinderaErrorKind::DictionaryKindError
                .with_error(anyhow::anyhow!("Invalid dictionary kind: {}", input))),
        }
    }
}
