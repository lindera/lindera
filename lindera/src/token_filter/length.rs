use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const LENGTH_TOKEN_FILTER_NAME: &str = "length";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LengthTokenFilterConfig {
    min: Option<usize>,
    max: Option<usize>,
}

impl LengthTokenFilterConfig {
    pub fn new(min: Option<usize>, max: Option<usize>) -> Self {
        Self { min, max }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Keep only tokens with the specified number of characters of text.
///
#[derive(Clone, Debug)]
pub struct LengthTokenFilter {
    config: LengthTokenFilterConfig,
}

impl LengthTokenFilter {
    pub fn new(config: LengthTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(LengthTokenFilterConfig::from_slice(data)?))
    }
}

impl TokenFilter for LengthTokenFilter {
    fn name(&self) -> &'static str {
        LENGTH_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        tokens.retain(|token| {
            let len = token.get_text().chars().count();
            if let Some(min) = self.config.min {
                if len < min {
                    return false;
                }
            }
            if let Some(max) = self.config.max {
                if len > max {
                    return false;
                }
            }
            true
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    use crate::token_filter::length::{LengthTokenFilter, LengthTokenFilterConfig};
    #[cfg(feature = "ipadic")]
    use crate::{builder, DictionaryKind, Token};

    #[test]
    fn test_length_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "min": 1,
            "max": 3
        }
        "#;
        let config = LengthTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.min.unwrap(), 1);
        assert_eq!(config.max.unwrap(), 3);

        let config_str = r#"
        {
            "min": 1
        }
        "#;
        let config = LengthTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.min.unwrap(), 1);
        assert_eq!(config.max, None);

        let config_str = r#"
        {
            "max": 2
        }
        "#;
        let config = LengthTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.min, None);
        assert_eq!(config.max.unwrap(), 2);
    }

    #[test]
    fn test_length_token_filter_from_slice() {
        let config_str = r#"
        {
            "min": 1,
            "max": 3
        }
        "#;
        let result = LengthTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_ok(), true);

        let config_str = r#"
        {
            "min": 1
        }
        "#;
        let result = LengthTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_ok(), true);

        let config_str = r#"
        {
            "max": 2
        }
        "#;
        let result = LengthTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_length_token_filter_apply_ipadic() {
        let config_str = r#"
        {
            "min": 3,
            "max": 3
        }
        "#;
        let filter = LengthTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].get_text(), "not");
        assert_eq!(tokens[1].get_text(), "the");
    }
}
