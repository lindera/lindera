use std::borrow::Cow;

use lindera_core::token_filter::TokenFilter;

use crate::{LinderaResult, Token};

pub const KOREAN_READING_FORM_TOKEN_FILTER_NAME: &str = "korean_reading_form";

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
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if let Some(details) = &token.details {
                if &details[0] == "UNK" {
                    // NOOP
                    continue;
                }
                token.text = Cow::Owned(details[3].clone());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{token_filter::korean_reading_form::KoreanReadingFormTokenFilter, Token};

    #[test]
    fn test_korean_reading_form_token_filter_apply() {
        let filter = KoreanReadingFormTokenFilter::default();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("한국어"),
                details: Some(vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "한국어".to_string(),
                    "Compound".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "한국/NNG/*+어/NNG/*".to_string(),
                ]),
                byte_start: 0,
                byte_end: 9,
            },
            Token {
                text: Cow::Borrowed("의"),
                details: Some(vec![
                    "JKG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "의".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
                byte_start: 9,
                byte_end: 12,
            },
            Token {
                text: Cow::Borrowed("형태"),
                details: Some(vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "형태".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
                byte_start: 12,
                byte_end: 18,
            },
            Token {
                text: Cow::Borrowed("해석"),
                details: Some(vec![
                    "NNG".to_string(),
                    "행위".to_string(),
                    "T".to_string(),
                    "해석".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
                byte_start: 18,
                byte_end: 24,
            },
            Token {
                text: Cow::Borrowed("을"),
                details: Some(vec![
                    "JKO".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "을".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
                byte_start: 24,
                byte_end: 27,
            },
            Token {
                text: Cow::Borrowed("실시"),
                details: Some(vec![
                    "NNG".to_string(),
                    "행위".to_string(),
                    "F".to_string(),
                    "실시".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
                byte_start: 27,
                byte_end: 33,
            },
            Token {
                text: Cow::Borrowed("할"),
                details: Some(vec![
                    "VV+ETM".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "할".to_string(),
                    "Inflect".to_string(),
                    "VV".to_string(),
                    "ETM".to_string(),
                    "하/VV/*+ᆯ/ETM/*".to_string(),
                ]),
                byte_start: 33,
                byte_end: 36,
            },
            Token {
                text: Cow::Borrowed("수"),
                details: Some(vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "수".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
                byte_start: 36,
                byte_end: 39,
            },
            Token {
                text: Cow::Borrowed("있"),
                details: Some(vec![
                    "VX".to_string(),
                    "*".to_string(),
                    "T".to_string(),
                    "있".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
                byte_start: 39,
                byte_end: 42,
            },
            Token {
                text: Cow::Borrowed("습니다"),
                details: Some(vec![
                    "EF".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "습니다".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
                byte_start: 42,
                byte_end: 51,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 10);
        assert_eq!(tokens[0].text, "한국어");
        assert_eq!(tokens[1].text, "의");
        assert_eq!(tokens[2].text, "형태");
        assert_eq!(tokens[3].text, "해석");
        assert_eq!(tokens[4].text, "을");
        assert_eq!(tokens[5].text, "실시");
        assert_eq!(tokens[6].text, "할");
        assert_eq!(tokens[7].text, "수");
        assert_eq!(tokens[8].text, "있");
        assert_eq!(tokens[9].text, "습니다");
    }
}
