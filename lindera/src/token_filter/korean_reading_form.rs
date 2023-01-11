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
    fn name(&self) -> &'static str {
        KOREAN_READING_FORM_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            match token.get_details() {
                Some(details) => {
                    if details[0] != "UNK" {
                        let new_text = details[3].to_string();
                        token.set_text(new_text);
                    }
                }
                None => continue,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ko-dic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    #[cfg(feature = "ko-dic")]
    use crate::{
        builder, token_filter::korean_reading_form::KoreanReadingFormTokenFilter, DictionaryKind,
        Token,
    };

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_korean_reading_form_token_filter_apply() {
        let filter = KoreanReadingFormTokenFilter::default();

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

        assert_eq!(tokens.len(), 10);
        assert_eq!(tokens[0].get_text(), "한국어");
        assert_eq!(tokens[1].get_text(), "의");
        assert_eq!(tokens[2].get_text(), "형태");
        assert_eq!(tokens[3].get_text(), "해석");
        assert_eq!(tokens[4].get_text(), "을");
        assert_eq!(tokens[5].get_text(), "실시");
        assert_eq!(tokens[6].get_text(), "할");
        assert_eq!(tokens[7].get_text(), "수");
        assert_eq!(tokens[8].get_text(), "있");
        assert_eq!(tokens[9].get_text(), "습니다");
    }
}
