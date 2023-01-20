use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME: &str = "korean_keep_tags";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct KoreanKeepTagsTokenFilterConfig {
    tags: HashSet<String>,
}

impl KoreanKeepTagsTokenFilterConfig {
    pub fn new(tags: HashSet<String>) -> Self {
        Self { tags }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<KoreanKeepTagsTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Keep only tokens with the specified part-of-speech tag.
///
#[derive(Clone, Debug)]
pub struct KoreanKeepTagsTokenFilter {
    config: KoreanKeepTagsTokenFilterConfig,
}

impl KoreanKeepTagsTokenFilter {
    pub fn new(config: KoreanKeepTagsTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(KoreanKeepTagsTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for KoreanKeepTagsTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        for token in tokens.iter_mut() {
            if let Some(details) = token.get_details() {
                if self.config.tags.contains(details[0]) {
                    new_tokens.push(token.clone());
                }
            }
        }

        mem::swap(tokens, &mut new_tokens);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ko-dic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    use crate::token_filter::korean_keep_tags::{
        KoreanKeepTagsTokenFilter, KoreanKeepTagsTokenFilterConfig,
    };
    #[cfg(feature = "ko-dic")]
    use crate::{builder, DictionaryKind, Token};

    #[test]
    fn test_korean_keep_tags_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "tags": [
                "NNG"
            ]
        }
        "#;
        let config = KoreanKeepTagsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.tags.len(), 1);
    }

    #[test]
    fn test_korean_keep_tags_token_filter_from_slice() {
        let config_str = r#"
        {
            "tags": [
                "NNG"
            ]
        }
        "#;
        let result = KoreanKeepTagsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_korean_keep_tags_token_filter_apply() {
        let config_str = r#"
        {
            "tags": [
                "NNG"
            ]
        }
        "#;
        let filter = KoreanKeepTagsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::KoDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("한국어", 0, 9, 0, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "한국어".to_string(),
                    "Compound".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "한국/NNG/*+어/NNG/*".to_string(),
                ]))
                .clone(),
            Token::new("의", 9, 12, 1, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "JKG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "의".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("형태", 12, 18, 2, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "형태".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("해석", 18, 24, 3, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "NNG".to_string(),
                    "행위".to_string(),
                    "T".to_string(),
                    "해석".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("을", 24, 27, 4, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "JKO".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "을".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("실시", 27, 33, 5, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "NNG".to_string(),
                    "행위".to_string(),
                    "F".to_string(),
                    "실시".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("할", 33, 36, 6, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "VV+ETM".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "할".to_string(),
                    "Inflect".to_string(),
                    "VV".to_string(),
                    "ETM".to_string(),
                    "하/VV/*+ᆯ/ETM/*".to_string(),
                ]))
                .clone(),
            Token::new("수", 36, 39, 7, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "수".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("있", 39, 42, 8, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "VX".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "있".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("습니다", 42, 51, 9, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "EF".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "습니다".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].get_text(), "한국어");
        assert_eq!(tokens[1].get_text(), "형태");
        assert_eq!(tokens[2].get_text(), "해석");
        assert_eq!(tokens[3].get_text(), "실시");
        assert_eq!(tokens[4].get_text(), "수");
    }
}
