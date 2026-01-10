use std::borrow::Cow;
use std::collections::HashMap;

use daachorse::DoubleArrayAhoCorasick;
use daachorse::DoubleArrayAhoCorasickBuilder;
use serde_json::Value;

use crate::LinderaResult;
use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const MAPPING_TOKEN_FILTER_NAME: &str = "mapping";

pub type MappingTokenFilterConfig = Value;

/// Replace characters with the specified character mappings.
///
#[derive(Clone)]
pub struct MappingTokenFilter {
    mapping: HashMap<String, String>,
    trie: DoubleArrayAhoCorasick<u32>,
}

impl MappingTokenFilter {
    pub fn new(mapping: HashMap<String, String>) -> LinderaResult<Self> {
        let mut keyset: Vec<(&[u8], u32)> = Vec::new();
        let mut keys = mapping.keys().collect::<Vec<_>>();
        keys.sort();
        for (value, key) in keys.into_iter().enumerate() {
            keyset.push((key.as_bytes(), value as u32));
        }

        let trie = DoubleArrayAhoCorasickBuilder::new()
            .build_with_values(keyset)
            .map_err(|err| LinderaErrorKind::Build.with_error(anyhow::anyhow!(err)))?;

        Ok(Self { mapping, trie })
    }

    pub fn from_config(config: &MappingTokenFilterConfig) -> LinderaResult<Self> {
        let mapping = config
            .get("mapping")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                LinderaErrorKind::Parse.with_error(anyhow::anyhow!("mapping must be an object."))
            })?
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect::<HashMap<String, String>>();

        Self::new(mapping)
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
            let len = token.surface.len();

            while start < len {
                let suffix = &token.surface[start..];
                match self
                    .trie
                    .find_overlapping_iter(suffix.as_bytes())
                    .filter(|m| m.start() == 0)
                    .last()
                    .map(|m| m.end())
                {
                    Some(prefix_len) => {
                        let surface = &token.surface[start..start + prefix_len];
                        let replacement = &self.mapping[surface];

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

            token.surface = Cow::Owned(result);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::mapping::{MappingTokenFilter, MappingTokenFilterConfig};

    #[test]
    fn test_mapping_token_filter_config() {
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
        let result: Result<MappingTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mapping_token_filter() {
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
        let config = serde_json::from_str::<MappingTokenFilterConfig>(config_str).unwrap();

        let result = MappingTokenFilter::from_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_mapping_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
        {
            "mapping": {
                "籠": "篭"
            }
        }
        "#;
        let config = serde_json::from_str::<MappingTokenFilterConfig>(config_str).unwrap();

        let filter = MappingTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                surface: Cow::Borrowed("籠原"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 312630,
                    is_system: true,
                    lex_type: LexType::System,
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
                surface: Cow::Borrowed("駅"),
                byte_start: 6,
                byte_end: 9,
                position: 1,
                position_length: 1,
                word_id: WordId {
                    id: 383791,
                    is_system: true,
                    lex_type: LexType::System,
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
        assert_eq!(&tokens[0].surface, "篭原");
        assert_eq!(&tokens[1].surface, "駅");
    }
}
