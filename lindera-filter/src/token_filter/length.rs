use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};

use crate::{token::FilteredToken, token_filter::TokenFilter};

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

    fn apply(&self, tokens: &mut Vec<FilteredToken>) -> LinderaResult<()> {
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
    use crate::{
        token::FilteredToken,
        token_filter::{
            length::{LengthTokenFilter, LengthTokenFilterConfig},
            TokenFilter,
        },
    };

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
    fn test_length_token_filter_apply_ipadic() {
        let config_str = r#"
        {
            "min": 3,
            "max": 3
        }
        "#;
        let filter = LengthTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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

        assert_eq!(tokens.len(), 2);
        assert_eq!(&tokens[0].text, "not");
        assert_eq!(&tokens[1].text, "the");
    }
}
