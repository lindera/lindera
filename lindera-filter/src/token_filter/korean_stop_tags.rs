use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};

use crate::{token::FilteredToken, token_filter::TokenFilter};

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

/// Remove tokens with the specified part-of-speech tag.
///
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

    fn apply(&self, tokens: &mut Vec<FilteredToken>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        for token in tokens.iter_mut() {
            if !self.config.tags.contains(&token.details[0]) {
                new_tokens.push(token.clone());
            }
        }

        mem::swap(tokens, &mut new_tokens);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::korean_stop_tags::{
        KoreanStopTagsTokenFilter, KoreanStopTagsTokenFilterConfig,
    };
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use crate::{token::FilteredToken, token_filter::TokenFilter};

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
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
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

        let mut tokens: Vec<FilteredToken> = vec![
            FilteredToken {
                text: "한국어".to_string(),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                details: vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "한국어".to_string(),
                    "Compound".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "한국/NNG/*+어/NNG/*".to_string(),
                ],
            },
            FilteredToken {
                text: "의".to_string(),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                details: vec![
                    "JKG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "의".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
            FilteredToken {
                text: "형태".to_string(),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                details: vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "형태".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
            FilteredToken {
                text: "해석".to_string(),
                byte_start: 18,
                byte_end: 24,
                position: 3,
                position_length: 1,
                details: vec![
                    "NNG".to_string(),
                    "행위".to_string(),
                    "T".to_string(),
                    "해석".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
            FilteredToken {
                text: "을".to_string(),
                byte_start: 24,
                byte_end: 27,
                position: 4,
                position_length: 1,
                details: vec![
                    "JKO".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "을".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
            FilteredToken {
                text: "실시".to_string(),
                byte_start: 27,
                byte_end: 33,
                position: 5,
                position_length: 1,
                details: vec![
                    "NNG".to_string(),
                    "행위".to_string(),
                    "F".to_string(),
                    "실시".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
            FilteredToken {
                text: "할".to_string(),
                byte_start: 33,
                byte_end: 36,
                position: 6,
                position_length: 1,
                details: vec![
                    "VV+ETM".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "할".to_string(),
                    "Inflect".to_string(),
                    "VV".to_string(),
                    "ETM".to_string(),
                    "하/VV/*+ᆯ/ETM/*".to_string(),
                ],
            },
            FilteredToken {
                text: "수".to_string(),
                byte_start: 36,
                byte_end: 39,
                position: 7,
                position_length: 1,
                details: vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "수".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
            FilteredToken {
                text: "있".to_string(),
                byte_start: 39,
                byte_end: 42,
                position: 8,
                position_length: 1,
                details: vec![
                    "VX".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "있".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
            FilteredToken {
                text: "습니다".to_string(),
                byte_start: 42,
                byte_end: 51,
                position: 9,
                position_length: 1,
                details: vec![
                    "EF".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "습니다".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 7);
        assert_eq!(&tokens[0].text, "한국어");
        assert_eq!(&tokens[1].text, "형태");
        assert_eq!(&tokens[2].text, "해석");
        assert_eq!(&tokens[3].text, "실시");
        assert_eq!(&tokens[4].text, "할");
        assert_eq!(&tokens[5].text, "수");
        assert_eq!(&tokens[6].text, "있");
    }
}
