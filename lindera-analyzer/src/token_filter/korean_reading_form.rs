use lindera_core::LinderaResult;

use crate::{token::Token, token_filter::TokenFilter};

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

    fn apply<'a>(&self, tokens: &mut Vec<Token>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if token.details[0] != "UNK" {
                token.text = token.details[3].to_string();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use lindera_core::word_entry::WordId;

    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use crate::{
        token::Token,
        token_filter::{korean_reading_form::KoreanReadingFormTokenFilter, TokenFilter},
    };

    #[test]
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    fn test_korean_reading_form_token_filter_apply() {
        let filter = KoreanReadingFormTokenFilter::default();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: "한국어".to_string(),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId(770060, true),
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
            Token {
                text: "의".to_string(),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId(576336, true),
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
            Token {
                text: "형태소".to_string(),
                byte_start: 12,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId(787807, true),
                details: vec![
                    "NNG".to_string(),
                    "*".to_string(),
                    "F".to_string(),
                    "형태소".to_string(),
                    "Compound".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "형태/NNG/*+소/NNG/*".to_string(),
                ],
            },
            Token {
                text: "분석".to_string(),
                byte_start: 21,
                byte_end: 27,
                position: 3,
                position_length: 1,
                word_id: WordId(383955, true),
                details: vec![
                    "NNG".to_string(),
                    "행위".to_string(),
                    "T".to_string(),
                    "분석".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ],
            },
            Token {
                text: "을".to_string(),
                byte_start: 27,
                byte_end: 30,
                position: 4,
                position_length: 1,
                word_id: WordId(574939, true),
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
            Token {
                text: "할".to_string(),
                byte_start: 30,
                byte_end: 33,
                position: 5,
                position_length: 1,
                word_id: WordId(774117, true),
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
            Token {
                text: "수".to_string(),
                byte_start: 33,
                byte_end: 36,
                position: 6,
                position_length: 1,
                word_id: WordId(444151, true),
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
            Token {
                text: "있".to_string(),
                byte_start: 36,
                byte_end: 39,
                position: 7,
                position_length: 1,
                word_id: WordId(602850, true),
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
            Token {
                text: "습니다".to_string(),
                byte_start: 39,
                byte_end: 48,
                position: 8,
                position_length: 1,
                word_id: WordId(458024, true),
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
