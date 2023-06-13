use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

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
            let len = token.text.chars().count();
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
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_core::word_entry::WordId;
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_tokenizer::token::Token;

    use crate::token_filter::length::{LengthTokenFilter, LengthTokenFilterConfig};
    #[cfg(any(all(feature = "ipadic",),))]
    use crate::token_filter::TokenFilter;

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
    #[cfg(any(all(feature = "ipadic",),))]
    fn test_length_token_filter_apply_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "min": 2,
                "max": 3
            }
            "#;
        let filter = LengthTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "すもも");
        assert_eq!(&tokens[1].text, "もも");
        assert_eq!(&tokens[2].text, "もも");
        assert_eq!(&tokens[3].text, "うち");
    }
}
