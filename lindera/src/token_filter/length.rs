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
    fn apply<'a>(&self, tokens: Vec<Token<'a>>) -> LinderaResult<Vec<Token<'a>>> {
        let mut t = Vec::new();
        for token in tokens.iter() {
            let len = token.text.chars().count();
            if let Some(min) = self.config.min {
                if len < min {
                    continue;
                }
            }

            if let Some(max) = self.config.max {
                if len > max {
                    continue;
                }
            }

            t.push(token.clone());
        }

        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use lindera_core::token_filter::TokenFilter;

    use crate::{
        token_filter::length::{LengthTokenFilter, LengthTokenFilterConfig},
        Token,
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
    fn test_length_token_filter_apply() {
        let config_str = r#"
        {
            "min": 3,
            "max": 3
        }
        "#;
        let filter = LengthTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let tokens: Vec<Token> = vec![
            Token {
                text: "to",
                details: None,
            },
            Token {
                text: "be",
                details: None,
            },
            Token {
                text: "or",
                details: None,
            },
            Token {
                text: "not",
                details: None,
            },
            Token {
                text: "to",
                details: None,
            },
            Token {
                text: "be",
                details: None,
            },
            Token {
                text: "this",
                details: None,
            },
            Token {
                text: "is",
                details: None,
            },
            Token {
                text: "the",
                details: None,
            },
            Token {
                text: "question",
                details: None,
            },
        ];

        let result = filter.apply(tokens).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "not");
        assert_eq!(result[1].text, "the");
    }
}
