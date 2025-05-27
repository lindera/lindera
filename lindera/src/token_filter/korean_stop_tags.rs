use std::collections::HashSet;

use serde_json::Value;

use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;
use crate::LinderaResult;

pub const KOREAN_STOP_TAGS_TOKEN_FILTER_NAME: &str = "korean_stop_tags";

pub type KoreanStopTagsTokenFilterConfig = Value;

/// Remove tokens with the specified part-of-speech tag.
///
#[derive(Clone, Debug)]
pub struct KoreanStopTagsTokenFilter {
    tags: HashSet<String>,
}

impl KoreanStopTagsTokenFilter {
    pub fn new(tags: HashSet<String>) -> Self {
        Self { tags }
    }

    pub fn from_config(config: &KoreanStopTagsTokenFilterConfig) -> LinderaResult<Self> {
        let tags: HashSet<String> = config["tags"]
            .as_array()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("tags is required"))
            })?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!("tag must be string"))
                    })
                    .map(|s| s.to_string())
            })
            .collect::<LinderaResult<HashSet<String>>>()?;

        Ok(Self::new(tags))
    }
}

impl TokenFilter for KoreanStopTagsTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_STOP_TAGS_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        // Create a new vector to store the filtered tokens
        let mut filtered_tokens = Vec::with_capacity(tokens.len());

        // Iterate over the tokens and filter them based on the part-of-speech tags in the config.
        for mut token in tokens.drain(..) {
            // Get the part-of-speech tags.
            let tag = token.get_detail(0).unwrap_or_default();

            // Add the token to the filtered tokens vector if the part-of-speech tag is not in the config.
            if !self.tags.contains(tag) {
                filtered_tokens.push(token);
            }
        }

        // Replace the original tokens vector with the filtered tokens.
        *tokens = filtered_tokens;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::korean_stop_tags::{
        KoreanStopTagsTokenFilter, KoreanStopTagsTokenFilterConfig,
    };

    #[test]
    fn test_korean_stop_tags_token_filter_config() {
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
        let result: Result<KoreanStopTagsTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
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
        let config: KoreanStopTagsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = KoreanStopTagsTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_korean_stop_tags_token_filter_apply() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;

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
        let config: KoreanStopTagsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = KoreanStopTagsTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::KoDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("한국어"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 770060,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("NNG"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("한국어"),
                    Cow::Borrowed("Compound"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("한국/NNG/*+어/NNG/*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("의"),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId {
                    id: 576336,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("JKG"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("의"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("형태소"),
                byte_start: 12,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId {
                    id: 787807,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("NNG"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("형태소"),
                    Cow::Borrowed("Compound"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("형태/NNG/*+소/NNG/*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("분석"),
                byte_start: 21,
                byte_end: 27,
                position: 3,
                position_length: 1,
                word_id: WordId {
                    id: 383955,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("NNG"),
                    Cow::Borrowed("행위"),
                    Cow::Borrowed("T"),
                    Cow::Borrowed("분석"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("을"),
                byte_start: 27,
                byte_end: 30,
                position: 4,
                position_length: 1,
                word_id: WordId {
                    id: 574939,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("JKO"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("T"),
                    Cow::Borrowed("을"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("할"),
                byte_start: 30,
                byte_end: 33,
                position: 5,
                position_length: 1,
                word_id: WordId {
                    id: 774117,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("VV+ETM"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("T"),
                    Cow::Borrowed("할"),
                    Cow::Borrowed("Inflect"),
                    Cow::Borrowed("VV"),
                    Cow::Borrowed("ETM"),
                    Cow::Borrowed("하/VV/*+ᆯ/ETM/*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("수"),
                byte_start: 33,
                byte_end: 36,
                position: 6,
                position_length: 1,
                word_id: WordId {
                    id: 444151,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("NNG"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("수"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("있"),
                byte_start: 36,
                byte_end: 39,
                position: 7,
                position_length: 1,
                word_id: WordId {
                    id: 602850,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("VX"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("T"),
                    Cow::Borrowed("있"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("습니다"),
                byte_start: 39,
                byte_end: 48,
                position: 8,
                position_length: 1,
                word_id: WordId {
                    id: 458024,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("EF"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("습니다"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 6);
        assert_eq!(&tokens[0].text, "한국어");
        assert_eq!(&tokens[1].text, "형태소");
        assert_eq!(&tokens[2].text, "분석");
        assert_eq!(&tokens[3].text, "할");
        assert_eq!(&tokens[4].text, "수");
        assert_eq!(&tokens[5].text, "있");
    }
}
