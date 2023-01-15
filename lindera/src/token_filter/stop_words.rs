use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

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

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        tokens.retain(|token| !self.config.words.contains(token.get_text()));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    use crate::token_filter::stop_words::{StopWordsTokenFilter, StopWordsTokenFilterConfig};
    #[cfg(feature = "ipadic")]
    use crate::{builder, DictionaryKind, Token};

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

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("to", 0, 2, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("be", 3, 5, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("or", 6, 8, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("not", 9, 12, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("to", 13, 15, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("be", 16, 18, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("this", 19, 23, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("is", 24, 26, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("the", 27, 30, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
            Token::new("question", 31, 39, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].get_text(), "question");
    }
}
