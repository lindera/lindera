use std::borrow::Cow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use yada::builder::DoubleArrayBuilder;
use yada::DoubleArray;

use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;
use crate::LinderaResult;

pub const MAPPING_TOKEN_FILTER_NAME: &str = "mapping";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MappingTokenFilterConfig {
    pub mapping: HashMap<String, String>,
}

impl MappingTokenFilterConfig {
    pub fn new(map: HashMap<String, String>) -> Self {
        Self { mapping: map }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<MappingTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }

    pub fn from_value(value: &serde_json::Value) -> LinderaResult<Self> {
        serde_json::from_value::<MappingTokenFilterConfig>(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Replace characters with the specified character mappings.
///
#[derive(Clone)]
pub struct MappingTokenFilter {
    config: MappingTokenFilterConfig,
    trie: DoubleArray<Vec<u8>>,
}

impl MappingTokenFilter {
    pub fn new(config: MappingTokenFilterConfig) -> LinderaResult<Self> {
        let mut keyset: Vec<(&[u8], u32)> = Vec::new();
        let mut keys = config.mapping.keys().collect::<Vec<_>>();
        keys.sort();
        for (value, key) in keys.into_iter().enumerate() {
            keyset.push((key.as_bytes(), value as u32));
        }

        let data = DoubleArrayBuilder::build(&keyset).ok_or_else(|| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!("DoubleArray build error."))
        })?;

        let trie = DoubleArray::new(data);

        Ok(Self { config, trie })
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Self::new(MappingTokenFilterConfig::from_slice(data)?)
    }
}

impl TokenFilter for MappingTokenFilter {
    fn name(&self) -> &'static str {
        MAPPING_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            let mut result = String::new();
            let mut start = 0_usize;
            let len = token.text.len();

            while start < len {
                let suffix = &token.text[start..];
                match self
                    .trie
                    .common_prefix_search(suffix.as_bytes())
                    .last()
                    .map(|(_offset_len, prefix_len)| prefix_len)
                {
                    Some(prefix_len) => {
                        let surface = &token.text[start..start + prefix_len];
                        let replacement = &self.config.mapping[surface];

                        result.push_str(replacement);

                        // move start offset
                        start += prefix_len;
                    }
                    None => {
                        match suffix.chars().next() {
                            Some(c) => {
                                result.push(c);

                                // move start offset
                                start += c.len_utf8();
                            }
                            None => break,
                        }
                    }
                }
            }

            token.text = Cow::Owned(result);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mapping_token_filter_config_from_slice() {
        use crate::token_filter::mapping::MappingTokenFilterConfig;
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let config = MappingTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();
        assert_eq!("ア", config.mapping.get("ｱ").unwrap());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mapping_token_filter_from_slice() {
        use crate::token_filter::mapping::MappingTokenFilter;

        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let result = MappingTokenFilter::from_slice(config_str.as_bytes());
        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mapping_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::mapping::MappingTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
        {
            "mapping": {
                "籠": "篭"
            }
        }
        "#;
        let filter = MappingTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("籠原"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 312630,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("籠原"),
                    Cow::Borrowed("カゴハラ"),
                    Cow::Borrowed("カゴハラ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("駅"),
                byte_start: 6,
                byte_end: 9,
                position: 1,
                position_length: 1,
                word_id: WordId {
                    id: 383791,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("駅"),
                    Cow::Borrowed("エキ"),
                    Cow::Borrowed("エキ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(&tokens[0].text, "篭原");
        assert_eq!(&tokens[1].text, "駅");
    }
}
