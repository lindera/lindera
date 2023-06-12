use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

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

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        tokens.retain(|token| self.config.words.contains(token.text.to_string().as_str()));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_core::word_entry::WordId;
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_tokenizer::token::Token;

    use crate::token_filter::keep_words::{KeepWordsTokenFilter, KeepWordsTokenFilterConfig};
    #[cfg(any(all(feature = "ipadic",),))]
    use crate::token_filter::TokenFilter;

    #[test]
    fn test_keep_words_token_filter_config_from_slice_ipadic() {
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
    fn test_keep_words_token_filter_from_slice_ipadic() {
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
    #[cfg(any(all(feature = "ipadic",),))]
    fn test_keep_words_token_filter_apply_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "words": [
                    "すもも",
                    "もも"
                ]
            }
            "#;
        let filter = KeepWordsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("すもも", 0, 9, 0, WordId(36165, true), &dictionary, None),
            Token::new("も", 9, 12, 1, WordId(73246, true), &dictionary, None),
            Token::new("もも", 12, 18, 2, WordId(74990, true), &dictionary, None),
            Token::new("も", 18, 21, 3, WordId(73246, true), &dictionary, None),
            Token::new("もも", 21, 27, 4, WordId(74990, true), &dictionary, None),
            Token::new("の", 27, 30, 5, WordId(55831, true), &dictionary, None),
            Token::new("うち", 30, 36, 6, WordId(8029, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "すもも");
        assert_eq!(&tokens[1].text, "もも");
        assert_eq!(&tokens[2].text, "もも");
    }
}
