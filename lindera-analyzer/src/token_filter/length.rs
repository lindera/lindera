use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};

use crate::{token::Token, token_filter::TokenFilter};

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

    fn apply<'a>(&self, tokens: &mut Vec<Token>) -> LinderaResult<()> {
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
    #[cfg(feature = "ipadic")]
    use lindera_core::word_entry::WordId;

    use crate::token_filter::length::{LengthTokenFilter, LengthTokenFilterConfig};
    #[cfg(feature = "ipadic")]
    use crate::{token::Token, token_filter::TokenFilter};

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
                "min": 2,
                "max": 3
            }
            "#;
        let filter = LengthTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: "すもも".to_string(),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId(36165, true),
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "すもも".to_string(),
                    "スモモ".to_string(),
                    "スモモ".to_string(),
                ],
            },
            Token {
                text: "も".to_string(),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId(73246, true),
                details: vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ],
            },
            Token {
                text: "もも".to_string(),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                word_id: WordId(74990, true),
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ],
            },
            Token {
                text: "も".to_string(),
                byte_start: 18,
                byte_end: 21,
                position: 3,
                position_length: 1,
                word_id: WordId(73246, true),
                details: vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ],
            },
            Token {
                text: "もも".to_string(),
                byte_start: 21,
                byte_end: 27,
                position: 4,
                position_length: 1,
                word_id: WordId(74990, true),
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ],
            },
            Token {
                text: "の".to_string(),
                byte_start: 27,
                byte_end: 30,
                position: 5,
                position_length: 1,
                word_id: WordId(55831, true),
                details: vec![
                    "助詞".to_string(),
                    "連体化".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "の".to_string(),
                    "ノ".to_string(),
                    "ノ".to_string(),
                ],
            },
            Token {
                text: "うち".to_string(),
                byte_start: 30,
                byte_end: 36,
                position: 6,
                position_length: 1,
                word_id: WordId(8029, true),
                details: vec![
                    "名詞".to_string(),
                    "非自立".to_string(),
                    "副詞可能".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "うち".to_string(),
                    "ウチ".to_string(),
                    "ウチ".to_string(),
                ],
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "すもも");
        assert_eq!(&tokens[1].text, "もも");
        assert_eq!(&tokens[2].text, "もも");
        assert_eq!(&tokens[3].text, "うち");
    }
}
