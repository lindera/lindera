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
            },
            Token {
                text: Cow::Borrowed("製"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("形態素"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("解析"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("器"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("Lindera"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("で"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("日本語"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("を"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("形態素"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("解析する。"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("する"),
                details: None,
            },
            Token {
                text: Cow::Borrowed("。"),
                details: None,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "Rust");
        assert_eq!(tokens[1].text, "Lindera");
    }
}
