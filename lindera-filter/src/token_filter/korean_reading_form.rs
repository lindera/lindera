use lindera_core::LinderaResult;

use crate::token::FilteredToken;

use super::TokenFilter;

pub const KOREAN_READING_FORM_TOKEN_FILTER_NAME: &str = "korean_reading_form";

/// Replace the text of a token with the reading of the text as registered in the morphological dictionary.
///
#[derive(Clone, Debug)]
pub struct KoreanReadingFormTokenFilter {}

impl KoreanReadingFormTokenFilter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for KoreanReadingFormTokenFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFilter for KoreanReadingFormTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_READING_FORM_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<FilteredToken>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if &token.details[0] != "UNK" {
                token.text = token.details[3].clone();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ko-dic")]
    use crate::{
        token::FilteredToken, token_filter::korean_reading_form::KoreanReadingFormTokenFilter,
        token_filter::TokenFilter,
    };

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_korean_reading_form_token_filter_apply() {
        let filter = KoreanReadingFormTokenFilter::default();

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

        assert_eq!(tokens.len(), 10);
        assert_eq!(&tokens[0].text, "한국어");
        assert_eq!(&tokens[1].text, "의");
        assert_eq!(&tokens[2].text, "형태");
        assert_eq!(&tokens[3].text, "해석");
        assert_eq!(&tokens[4].text, "을");
        assert_eq!(&tokens[5].text, "실시");
        assert_eq!(&tokens[6].text, "할");
        assert_eq!(&tokens[7].text, "수");
        assert_eq!(&tokens[8].text, "있");
        assert_eq!(&tokens[9].text, "습니다");
    }
}
