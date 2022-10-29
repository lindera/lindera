use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const STOP_WORDS_TOKEN_FILTER_NAME: &str = "stop_words";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StopWordsTokenFilterConfig {
    stop_words: HashSet<String>,
}

impl StopWordsTokenFilterConfig {
    pub fn new(stop_words: HashSet<String>) -> Self {
        Self { stop_words }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone, Debug)]
pub struct StopWordsTokenFilter {
    config: StopWordsTokenFilterConfig,
}

impl StopWordsTokenFilter {
    pub fn new(config: StopWordsTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(StopWordsTokenFilterConfig::from_slice(data)?))
    }
}

impl TokenFilter for StopWordsTokenFilter {
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        tokens.retain(|token| !self.config.stop_words.contains(token.text.as_ref()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{
        token_filter::stop_words::{StopWordsTokenFilter, StopWordsTokenFilterConfig},
        Token,
    };

    #[test]
    fn test_stop_words_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "stop_words": [
                "a",
                "an",
                "and",
                "are",
                "as",
                "at",
                "be",
                "but",
                "by",
                "for",
                "if",
                "in",
                "into",
                "is",
                "it",
                "no",
                "not",
                "of",
                "on",
                "or",
                "such",
                "that",
                "the",
                "their",
                "then",
                "there",
                "these",
                "they",
                "this",
                "to",
                "was",
                "will",
                "with"
            ]
        }
        "#;
        let config = StopWordsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.stop_words.len(), 33);
    }

    #[test]
    fn test_stop_words_token_filter_from_slice() {
        let config_str = r#"
        {
            "stop_words": [
                "a",
                "an",
                "and",
                "are",
                "as",
                "at",
                "be",
                "but",
                "by",
                "for",
                "if",
                "in",
                "into",
                "is",
                "it",
                "no",
                "not",
                "of",
                "on",
                "or",
                "such",
                "that",
                "the",
                "their",
                "then",
                "there",
                "these",
                "they",
                "this",
                "to",
                "was",
                "will",
                "with"
            ]
        }
        "#;
        let result = StopWordsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_stop_words_token_filter_apply() {
        let config_str = r#"
        {
            "stop_words": [
                "a",
                "an",
                "and",
                "are",
                "as",
                "at",
                "be",
                "but",
                "by",
                "for",
                "if",
                "in",
                "into",
                "is",
                "it",
                "no",
                "not",
                "of",
                "on",
                "or",
                "such",
                "that",
                "the",
                "their",
                "then",
                "there",
                "these",
                "they",
                "this",
                "to",
                "was",
                "will",
                "with"
            ]
        }
        "#;
        let filter = StopWordsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("to"),
                details: None,
                byte_start: 0,
                byte_end: 2,
            },
            Token {
                text: Cow::Borrowed("be"),
                details: None,
                byte_start: 3,
                byte_end: 5,
            },
            Token {
                text: Cow::Borrowed("or"),
                details: None,
                byte_start: 6,
                byte_end: 8,
            },
            Token {
                text: Cow::Borrowed("not"),
                details: None,
                byte_start: 9,
                byte_end: 12,
            },
            Token {
                text: Cow::Borrowed("to"),
                details: None,
                byte_start: 13,
                byte_end: 15,
            },
            Token {
                text: Cow::Borrowed("be"),
                details: None,
                byte_start: 16,
                byte_end: 18,
            },
            Token {
                text: Cow::Borrowed("this"),
                details: None,
                byte_start: 19,
                byte_end: 23,
            },
            Token {
                text: Cow::Borrowed("is"),
                details: None,
                byte_start: 24,
                byte_end: 26,
            },
            Token {
                text: Cow::Borrowed("the"),
                details: None,
                byte_start: 27,
                byte_end: 30,
            },
            Token {
                text: Cow::Borrowed("question"),
                details: None,
                byte_start: 31,
                byte_end: 39,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "question");
    }
}
