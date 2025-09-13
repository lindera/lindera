use std::borrow::Cow;

use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

use crate::LinderaResult;
use crate::character_filter::unicode_normalize::UnicodeNormalizeKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const REMOVE_DIACRITICAL_TOKEN_FILTER_NAME: &str = "remove_diacritical_mark";

pub type RemoveDiacriticalMarkTokenFilterConfig = Value;

fn get_normalize_kind(text: &str) -> UnicodeNormalizeKind {
    if text.nfc().eq(text.chars()) {
        UnicodeNormalizeKind::NFC
    } else if text.nfd().eq(text.chars()) {
        UnicodeNormalizeKind::NFD
    } else if text.nfkc().eq(text.chars()) {
        UnicodeNormalizeKind::NFKC
    } else if text.nfkd().eq(text.chars()) {
        UnicodeNormalizeKind::NFKD
    } else {
        UnicodeNormalizeKind::NFD
    }
}

/// Removes diacritics from token text.
///
#[derive(Clone, Debug)]
pub struct RemoveDiacriticalMarkTokenFilter {
    japanese: bool,
}

impl RemoveDiacriticalMarkTokenFilter {
    pub fn new(japanese: bool) -> Self {
        Self { japanese }
    }

    pub fn from_config(config: &RemoveDiacriticalMarkTokenFilterConfig) -> LinderaResult<Self> {
        let japanese = config
            .get("japanese")
            .is_some_and(|v| v.as_bool().unwrap_or(false));

        Ok(Self::new(japanese))
    }

    fn is_diacritic(&self, x: char) -> bool {
        ('\u{0300}'..='\u{036f}').contains(&x)
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

            if self.japanese && self.is_japanese_diacritic(x) {
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
            token.surface = Cow::Owned(self.remove_diacritic(token.surface.as_ref()));
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
    fn test_remove_diacritical_mark_token_filter_config() {
        let config_str = r#"
        {
            "japanese": true
        }
        "#;
        let result: Result<RemoveDiacriticalMarkTokenFilterConfig, _> =
            serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_remove_diacritical_mark_token_filter() {
        let config_str = r#"
        {
            "japanese": true
        }
        "#;
        let config: RemoveDiacriticalMarkTokenFilterConfig =
            serde_json::from_str(config_str).unwrap();
        let result = RemoveDiacriticalMarkTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embedded-ipadic")]
    fn test_remove_diacritical_token_filter_apply() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
        {
            "japanese": false
        }
        "#;
        let config: RemoveDiacriticalMarkTokenFilterConfig =
            serde_json::from_str(config_str).unwrap();
        let filter = RemoveDiacriticalMarkTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed("café"),
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

            assert_eq!(tokens[0].surface, "cafe");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed("ガソリン"),
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

            assert_eq!(tokens[0].surface, "ガソリン");
        }
    }

    #[test]
    #[cfg(feature = "embedded-ipadic")]
    fn test_remove_diacritical_token_filter_apply_japanese() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
        {
            "japanese": true
        }
        "#;
        let config: RemoveDiacriticalMarkTokenFilterConfig =
            serde_json::from_str(config_str).unwrap();
        let filter = RemoveDiacriticalMarkTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed("café"),
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

            assert_eq!(tokens[0].surface, "cafe");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed("ガソリン"),
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

            assert_eq!(tokens[0].surface, "カソリン");
        }
    }
}
