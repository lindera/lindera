use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};

use crate::{token::FilteredToken, token_filter::TokenFilter};

pub const KEEP_WORDS_TOKEN_FILTER_NAME: &str = "keep_words";
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct KeepWordsTokenFilterConfig {
    words: HashSet<String>,
}

impl KeepWordsTokenFilterConfig {
    pub fn new(words: HashSet<String>) -> Self {
        Self { words }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Keep only the tokens of the specified text.
///
#[derive(Clone, Debug)]
pub struct KeepWordsTokenFilter {
    config: KeepWordsTokenFilterConfig,
}

impl KeepWordsTokenFilter {
    pub fn new(config: KeepWordsTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(KeepWordsTokenFilterConfig::from_slice(data)?))
    }
}

impl TokenFilter for KeepWordsTokenFilter {
    fn name(&self) -> &'static str {
        KEEP_WORDS_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<FilteredToken>) -> LinderaResult<()> {
        tokens.retain(|token| self.config.words.contains(&token.text));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        token::FilteredToken,
        token_filter::{
            keep_words::{KeepWordsTokenFilter, KeepWordsTokenFilterConfig},
            TokenFilter,
        },
    };

    #[test]
    fn test_keep_words_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "words": [
                "すもも",
                "もも"
            ]
        }
        "#;
        let config = KeepWordsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.words.len(), 2);
    }

    #[test]
    fn test_keep_words_token_filter_from_slice() {
        let config_str = r#"
        {
            "words": [
                "すもも",
                "もも"
            ]
        }
        "#;
        let result = KeepWordsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_keep_words_token_filter_apply_ipadic() {
        let config_str = r#"
        {
            "words": [
                "すもも",
                "もも"
            ]
        }
        "#;
        let filter = KeepWordsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<FilteredToken> = vec![
            FilteredToken {
                text: "すもも".to_string(),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "すもも".to_string(),
                    "スモモ".to_string(),
                    "スモモ".to_string(),
                ],
            },
            FilteredToken {
                text: "も".to_string(),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                details: vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ],
            },
            FilteredToken {
                text: "もも".to_string(),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ],
            },
            FilteredToken {
                text: "も".to_string(),
                byte_start: 18,
                byte_end: 21,
                position: 3,
                position_length: 1,
                details: vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ],
            },
            FilteredToken {
                text: "もも".to_string(),
                byte_start: 21,
                byte_end: 27,
                position: 4,
                position_length: 1,
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ],
            },
            FilteredToken {
                text: "の".to_string(),
                byte_start: 27,
                byte_end: 30,
                position: 5,
                position_length: 1,
                details: vec![
                    "助詞".to_string(),
                    "連体化".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "の".to_string(),
                    "ノ".to_string(),
                    "ノ".to_string(),
                ],
            },
            FilteredToken {
                text: "うち".to_string(),
                byte_start: 30,
                byte_end: 36,
                position: 6,
                position_length: 1,
                details: vec![
                    "名詞".to_string(),
                    "非自立".to_string(),
                    "副詞可能".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "うち".to_string(),
                    "ウチ".to_string(),
                    "ウチ".to_string(),
                ],
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "すもも");
        assert_eq!(&tokens[1].text, "もも");
        assert_eq!(&tokens[2].text, "もも");
    }
}
