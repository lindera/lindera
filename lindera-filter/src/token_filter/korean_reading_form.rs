use lindera_core::LinderaResult;
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

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

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if let Some(details) = &mut token.get_details() {
                if details[0] != "UNK" {
                    token.text = details[3].to_string().into();
                }
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
    use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use lindera_tokenizer::token::Token;

    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use crate::token_filter::{korean_reading_form::KoreanReadingFormTokenFilter, TokenFilter};

    #[test]
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    fn test_korean_reading_form_token_filter_apply() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let filter = KoreanReadingFormTokenFilter::default();

        let mut tokens: Vec<Token> = vec![
            Token::new("한국어", 0, 9, 0, WordId(770060, true), &dictionary, None),
            Token::new("의", 9, 12, 1, WordId(576336, true), &dictionary, None),
            Token::new("형태소", 12, 21, 2, WordId(787807, true), &dictionary, None),
            Token::new("분석", 21, 27, 3, WordId(383955, true), &dictionary, None),
            Token::new("을", 27, 30, 4, WordId(574939, true), &dictionary, None),
            Token::new("할", 30, 33, 5, WordId(774117, true), &dictionary, None),
            Token::new("수", 33, 36, 6, WordId(444151, true), &dictionary, None),
            Token::new("있", 36, 39, 6, WordId(602850, true), &dictionary, None),
            Token::new("습니다", 39, 48, 6, WordId(458024, true), &dictionary, None),
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
