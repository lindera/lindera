use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const KOREAN_STOP_TAGS_TOKEN_FILTER_NAME: &str = "korean_stop_tags";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct KoreanStopTagsTokenFilterConfig {
    tags: HashSet<String>,
}

impl KoreanStopTagsTokenFilterConfig {
    pub fn new(tags: HashSet<String>) -> Self {
        Self { tags }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<KoreanStopTagsTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone, Debug)]
pub struct KoreanStopTagsTokenFilter {
    config: KoreanStopTagsTokenFilterConfig,
}

impl KoreanStopTagsTokenFilter {
    pub fn new(config: KoreanStopTagsTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(KoreanStopTagsTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for KoreanStopTagsTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_STOP_TAGS_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        for token in tokens.iter_mut() {
            if let Some(details) = token.get_details() {
                if !self.config.tags.contains(details[0]) {
                    new_tokens.push(token.clone());
                }
            }
        }

        // tokens.retain(|mut token| {
        //     if let Some(details) = token.get_details() {
        //         !self.config.tags.contains(&details[0])
        //     } else {
        //         false
        //     }
        // });

        mem::swap(tokens, &mut new_tokens);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ko-dic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    use crate::token_filter::korean_stop_tags::{
        KoreanStopTagsTokenFilter, KoreanStopTagsTokenFilterConfig,
    };
    #[cfg(feature = "ko-dic")]
    use crate::{builder, DictionaryKind, Token};

    #[test]
    fn test_korean_stop_tags_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "tags": [
                "EP",
                "EF",
                "EC",
                "ETN",
                "ETM",
                "IC",
                "JKS",
                "JKC",
                "JKG",
                "JKO",
                "JKB",
                "JKV",
                "JKQ",
                "JX",
                "JC",
                "MAG",
                "MAJ",
                "MM",
                "SP",
                "SSC",
                "SSO",
                "SC",
                "SE",
                "XPN",
                "XSA",
                "XSN",
                "XSV",
                "UNA",
                "NA",
                "VSV"
            ]
        }
        "#;
        let config = KoreanStopTagsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.tags.len(), 30);
    }

    #[test]
    fn test_korean_stop_tagss_token_filter_from_slice() {
        let config_str = r#"
        {
            "tags": [
                "EP",
                "EF",
                "EC",
                "ETN",
                "ETM",
                "IC",
                "JKS",
                "JKC",
                "JKG",
                "JKO",
                "JKB",
                "JKV",
                "JKQ",
                "JX",
                "JC",
                "MAG",
                "MAJ",
                "MM",
                "SP",
                "SSC",
                "SSO",
                "SC",
                "SE",
                "XPN",
                "XSA",
                "XSN",
                "XSV",
                "UNA",
                "NA",
                "VSV"
            ]
        }
        "#;
        let result = KoreanStopTagsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_korean_stop_tags_token_filter_apply() {
        let config_str = r#"
        {
            "tags": [
                "EP",
                "EF",
                "EC",
                "ETN",
                "ETM",
                "IC",
                "JKS",
                "JKC",
                "JKG",
                "JKO",
                "JKB",
                "JKV",
                "JKQ",
                "JX",
                "JC",
                "MAG",
                "MAJ",
                "MM",
                "SP",
                "SSC",
                "SSO",
                "SC",
                "SE",
                "XPN",
                "XSA",
                "XSN",
                "XSV",
                "UNA",
                "NA",
                "VSV"
            ]
        }
        "#;
        let filter = KoreanStopTagsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::KoDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("한국어", 0, 9, WordId::default(), &dictionary, None)
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
            Token::new("의", 9, 12, WordId::default(), &dictionary, None)
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
            Token::new("형태", 12, 18, WordId::default(), &dictionary, None)
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
            Token::new("해석", 18, 24, WordId::default(), &dictionary, None)
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
            Token::new("을", 24, 27, WordId::default(), &dictionary, None)
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
            Token::new("실시", 27, 33, WordId::default(), &dictionary, None)
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
            Token::new("할", 33, 36, WordId::default(), &dictionary, None)
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
            Token::new("수", 36, 39, WordId::default(), &dictionary, None)
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
            Token::new("있", 39, 42, WordId::default(), &dictionary, None)
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
            Token::new("습니다", 42, 51, WordId::default(), &dictionary, None)
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

        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0].get_text(), "한국어");
        assert_eq!(tokens[1].get_text(), "형태");
        assert_eq!(tokens[2].get_text(), "해석");
        assert_eq!(tokens[3].get_text(), "실시");
        assert_eq!(tokens[4].get_text(), "할");
        assert_eq!(tokens[5].get_text(), "수");
        assert_eq!(tokens[6].get_text(), "있");
    }
}
