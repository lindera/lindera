use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

use crate::character_filter::unicode_normalize::UnicodeNormalizeKind;
use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;
use crate::LinderaResult;

pub const REMOVE_DIACRITICAL_TOKEN_FILTER_NAME: &str = "remove_diacritical_mark";

fn get_normalize_kind(text: &str) -> UnicodeNormalizeKind {
    if text.nfc().eq(text.chars()) {
        return UnicodeNormalizeKind::NFC;
    } else if text.nfd().eq(text.chars()) {
        return UnicodeNormalizeKind::NFD;
    } else if text.nfkc().eq(text.chars()) {
        return UnicodeNormalizeKind::NFKC;
    } else if text.nfkd().eq(text.chars()) {
        return UnicodeNormalizeKind::NFKD;
    } else {
        return UnicodeNormalizeKind::NFD;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RemoveDiacriticalMarkTokenFilterConfig {
    pub japanese: bool,
}

impl RemoveDiacriticalMarkTokenFilterConfig {
    pub fn new(japanese: bool) -> Self {
        Self { japanese }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<RemoveDiacriticalMarkTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        serde_json::from_value::<RemoveDiacriticalMarkTokenFilterConfig>(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Removes diacritics from token text.
///
#[derive(Clone, Debug)]
pub struct RemoveDiacriticalMarkTokenFilter {
    config: RemoveDiacriticalMarkTokenFilterConfig,
}

impl RemoveDiacriticalMarkTokenFilter {
    pub fn new(config: RemoveDiacriticalMarkTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(
            RemoveDiacriticalMarkTokenFilterConfig::from_slice(data)?,
        ))
    }

    fn is_diacritic(&self, x: char) -> bool {
        '\u{0300}' <= x && x <= '\u{036f}'
    }

    fn is_japanese_diacritic(&self, x: char) -> bool {
        x == '\u{3099}'  // japanese dakuten 
            || x == '\u{309b}'  // japanese dakuten
            || x == '\u{ff9e}'  // japanese dakuten
            || x == '\u{309c}'  // japanese han-dakuten
            || x == '\u{309a}'  // japanese han-dakuten
            || x == '\u{ff9f}' // japanese han-dakuten
    }

    fn remove_diacritic(&self, segment: &str) -> String {
        let original_kind = get_normalize_kind(segment);

        let mut normalized = String::with_capacity(128);

        for x in segment.nfd() {
            if self.is_diacritic(x) {
                continue;
            }

            if self.config.japanese && self.is_japanese_diacritic(x) {
                continue;
            }

            normalized.push(x);
        }

        match original_kind {
            UnicodeNormalizeKind::NFC => normalized.nfc().to_string(),
            UnicodeNormalizeKind::NFD => normalized.nfd().to_string(),
            UnicodeNormalizeKind::NFKC => normalized.nfkc().to_string(),
            UnicodeNormalizeKind::NFKD => normalized.nfkd().to_string(),
        }
    }
}

impl TokenFilter for RemoveDiacriticalMarkTokenFilter {
    fn name(&self) -> &'static str {
        REMOVE_DIACRITICAL_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.text = Cow::Owned(self.remove_diacritic(token.text.as_ref()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::remove_diacritical_mark::{
        RemoveDiacriticalMarkTokenFilter, RemoveDiacriticalMarkTokenFilterConfig,
    };

    #[test]
    fn test_remove_diacritical_mark_token_filter_config_new() {
        let config = RemoveDiacriticalMarkTokenFilterConfig::new(true);
        assert!(config.japanese);
    }

    #[test]
    fn test_remove_diacritical_mark_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "japanese": true
        }
        "#;
        let config =
            RemoveDiacriticalMarkTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();
        assert!(config.japanese);
    }

    #[test]
    fn test_japanese_iteration_mark_character_filter_from_slice() {
        let config_str = r#"
        {
            "japanese": true
        }
        "#;
        let result = RemoveDiacriticalMarkTokenFilter::from_slice(config_str.as_bytes());
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_remove_diacritical_token_filter_apply() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
        {
            "japanese": false
        }
        "#;
        let filter = RemoveDiacriticalMarkTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![Token {
                text: Cow::Borrowed("café"),
                byte_start: 0,
                byte_end: 4,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 4294967295,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![Cow::Borrowed("UNK")]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens[0].text, "cafe");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                text: Cow::Borrowed("ガソリン"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 84915,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("ガソリン"),
                    Cow::Borrowed("ガソリン"),
                    Cow::Borrowed("ガソリン"),
                ]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens[0].text, "ガソリン");
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_remove_diacritical_token_filter_apply_japanese() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
        {
            "japanese": true
        }
        "#;
        let filter = RemoveDiacriticalMarkTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![Token {
                text: Cow::Borrowed("café"),
                byte_start: 0,
                byte_end: 4,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 4294967295,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![Cow::Borrowed("UNK")]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens[0].text, "cafe");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                text: Cow::Borrowed("ガソリン"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 84915,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("ガソリン"),
                    Cow::Borrowed("ガソリン"),
                    Cow::Borrowed("ガソリン"),
                ]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens[0].text, "カソリン");
        }
    }
}
