use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};

use crate::token::FilteredToken;

use super::TokenFilter;

pub const STOP_WORDS_TOKEN_FILTER_NAME: &str = "stop_words";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StopWordsTokenFilterConfig {
    words: HashSet<String>,
}

impl StopWordsTokenFilterConfig {
    pub fn new(words: HashSet<String>) -> Self {
        Self { words }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Remove the tokens of the specified text.
///
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
    fn name(&self) -> &'static str {
        STOP_WORDS_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<FilteredToken>) -> LinderaResult<()> {
        tokens.retain(|token| !self.config.words.contains(&token.text));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::stop_words::{StopWordsTokenFilter, StopWordsTokenFilterConfig};

    #[cfg(feature = "ipadic")]
    use crate::{token::FilteredToken, token_filter::TokenFilter};

    #[test]
    fn test_stop_words_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "words": [
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

        assert_eq!(config.words.len(), 33);
    }

    #[test]
    fn test_stop_words_token_filter_from_slice() {
        let config_str = r#"
        {
            "words": [
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
    #[cfg(feature = "ipadic")]
    fn test_stop_words_token_filter_apply_ipadic() {
        let config_str = r#"
        {
            "words": [
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

        let mut tokens: Vec<FilteredToken> = vec![
            FilteredToken {
                text: "to".to_string(),
                byte_start: 0,
                byte_end: 2,
                position: 0,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "be".to_string(),
                byte_start: 3,
                byte_end: 5,
                position: 1,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "or".to_string(),
                byte_start: 6,
                byte_end: 8,
                position: 2,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "not".to_string(),
                byte_start: 9,
                byte_end: 12,
                position: 3,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "to".to_string(),
                byte_start: 13,
                byte_end: 15,
                position: 4,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "be".to_string(),
                byte_start: 16,
                byte_end: 18,
                position: 5,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "this".to_string(),
                byte_start: 19,
                byte_end: 23,
                position: 6,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "is".to_string(),
                byte_start: 24,
                byte_end: 26,
                position: 7,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "the".to_string(),
                byte_start: 27,
                byte_end: 30,
                position: 8,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
            FilteredToken {
                text: "question".to_string(),
                byte_start: 31,
                byte_end: 39,
                position: 9,
                position_length: 1,
                details: vec!["UNK".to_string()],
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].text, "question");
    }
}
