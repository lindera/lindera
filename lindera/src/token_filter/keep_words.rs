use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const KEEP_WORDS_TOKEN_FILTER_NAME: &str = "keep_words";
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct KeepWordsTokenFilterConfig {
    keep_words: HashSet<String>,
}

impl KeepWordsTokenFilterConfig {
    pub fn new(keep_words: HashSet<String>) -> Self {
        Self { keep_words }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

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
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        tokens.retain(|token| self.config.keep_words.contains(token.text.as_ref()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{
        token_filter::keep_words::{KeepWordsTokenFilter, KeepWordsTokenFilterConfig},
        Token,
    };

    #[test]
    fn test_keep_words_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "keep_words": [
                "Lindera",
                "Rust"
            ]
        }
        "#;
        let config = KeepWordsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.keep_words.len(), 2);
    }

    #[test]
    fn test_keep_words_token_filter_from_slice() {
        let config_str = r#"
        {
            "keep_words": [
                "Lindera",
                "Rust"
            ]
        }
        "#;
        let result = KeepWordsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_keep_words_token_filter_apply() {
        let config_str = r#"
        {
            "keep_words": [
                "Lindera",
                "Rust"
            ]
        }
        "#;
        let filter = KeepWordsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("Rust"),
                details: None,
                byte_start: 0,
                byte_end: 4,
            },
            Token {
                text: Cow::Borrowed("製"),
                details: None,
                byte_start: 4,
                byte_end: 7,
            },
            Token {
                text: Cow::Borrowed("形態素"),
                details: None,
                byte_start: 7,
                byte_end: 16,
            },
            Token {
                text: Cow::Borrowed("解析"),
                details: None,
                byte_start: 16,
                byte_end: 22,
            },
            Token {
                text: Cow::Borrowed("器"),
                details: None,
                byte_start: 22,
                byte_end: 25,
            },
            Token {
                text: Cow::Borrowed("Lindera"),
                details: None,
                byte_start: 25,
                byte_end: 32,
            },
            Token {
                text: Cow::Borrowed("で"),
                details: None,
                byte_start: 32,
                byte_end: 35,
            },
            Token {
                text: Cow::Borrowed("日本語"),
                details: None,
                byte_start: 35,
                byte_end: 44,
            },
            Token {
                text: Cow::Borrowed("を"),
                details: None,
                byte_start: 44,
                byte_end: 47,
            },
            Token {
                text: Cow::Borrowed("形態素"),
                details: None,
                byte_start: 47,
                byte_end: 56,
            },
            Token {
                text: Cow::Borrowed("解析"),
                details: None,
                byte_start: 56,
                byte_end: 62,
            },
            Token {
                text: Cow::Borrowed("する"),
                details: None,
                byte_start: 62,
                byte_end: 68,
            },
            Token {
                text: Cow::Borrowed("。"),
                details: None,
                byte_start: 68,
                byte_end: 71,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "Rust");
        assert_eq!(tokens[1].text, "Lindera");
    }
}
