use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::{TokenFilter, TokenFilterConfig};
use crate::LinderaResult;

pub const KOREAN_READING_FORM_TOKEN_FILTER_NAME: &str = "korean_reading_form";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct KoreanReadingFormTokenFilterConfig {}

impl KoreanReadingFormTokenFilterConfig {
    pub fn new() -> Self {
        Self {}
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<KoreanReadingFormTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

impl TokenFilterConfig for KoreanReadingFormTokenFilterConfig {
    fn from_value(value: &Value) -> LinderaResult<Self>
    where
        Self: Sized,
    {
        serde_json::from_value(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

impl Default for KoreanReadingFormTokenFilterConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Replace the text of a token with the reading of the text as registered in the morphological dictionary.
///
#[derive(Clone, Debug)]
pub struct KoreanReadingFormTokenFilter {
    #[allow(dead_code)]
    config: KoreanReadingFormTokenFilterConfig,
}

impl KoreanReadingFormTokenFilter {
    pub fn new(config: KoreanReadingFormTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(KoreanReadingFormTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for KoreanReadingFormTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_READING_FORM_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if let Some(detail) = token.get_detail(0) {
                if detail == "UNK" {
                    continue;
                }
            }

            if let Some(detail) = token.get_detail(3) {
                token.text = Cow::Owned(detail.to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_korean_reading_form_token_filter_apply() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::korean_reading_form::KoreanReadingFormTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {}
            "#;

        let filter = KoreanReadingFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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

        assert_eq!(tokens.len(), 9);
        assert_eq!(&tokens[0].text, "한국어");
        assert_eq!(&tokens[1].text, "의");
        assert_eq!(&tokens[2].text, "형태소");
        assert_eq!(&tokens[3].text, "분석");
        assert_eq!(&tokens[4].text, "을");
        assert_eq!(&tokens[5].text, "할");
        assert_eq!(&tokens[6].text, "수");
        assert_eq!(&tokens[7].text, "있");
        assert_eq!(&tokens[8].text, "습니다");
    }
}
