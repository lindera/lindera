#[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic"))]
use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::dictionary::DictionaryKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const JAPANESE_READING_FORM_TOKEN_FILTER_NAME: &str = "japanese_reading_form";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseReadingFormTokenFilterConfig {
    kind: DictionaryKind,
}

impl JapaneseReadingFormTokenFilterConfig {
    pub fn new(kind: DictionaryKind) -> Self {
        Self { kind }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<JapaneseReadingFormTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }

    pub fn from_value(value: &serde_json::Value) -> LinderaResult<Self> {
        serde_json::from_value::<JapaneseReadingFormTokenFilterConfig>(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Replace the text of a token with the reading of the text as registered in the morphological dictionary.
/// The reading is in katakana.
///
#[derive(Clone, Debug)]
pub struct JapaneseReadingFormTokenFilter {
    config: JapaneseReadingFormTokenFilterConfig,
}

impl JapaneseReadingFormTokenFilter {
    pub fn new(config: JapaneseReadingFormTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(JapaneseReadingFormTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for JapaneseReadingFormTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_READING_FORM_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if let Some(detail) = token.get_detail(0) {
                if detail == "UNK" {
                    continue;
                }
            }

            match self.config.kind {
                #[cfg(feature = "ipadic")]
                DictionaryKind::IPADIC => {
                    if let Some(detail) = token.get_detail(7) {
                        token.text = Cow::Owned(detail.to_string());
                    }
                }
                #[cfg(feature = "ipadic-neologd")]
                DictionaryKind::IPADICNEologd => {
                    if let Some(detail) = token.get_detail(7) {
                        token.text = Cow::Owned(detail.to_string());
                    }
                }
                #[cfg(feature = "unidic")]
                DictionaryKind::UniDic => {
                    if let Some(detail) = token.get_detail(6) {
                        token.text = Cow::Owned(detail.to_string());
                    }
                }
                _ => {
                    // NOOP
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_reading_form_token_filter_config_from_slice_ipadic() {
        use crate::dictionary::DictionaryKind;
        use crate::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilterConfig;

        let config_str = r#"
            {
                "kind": "ipadic"
            }
            "#;
        let config =
            JapaneseReadingFormTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, DictionaryKind::IPADIC);
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_reading_form_token_filter_config_from_slice_unidic() {
        use crate::dictionary::DictionaryKind;
        use crate::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilterConfig;

        let config_str = r#"
            {
                "kind": "unidic"
            }
            "#;
        let config =
            JapaneseReadingFormTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, DictionaryKind::UniDic);
    }

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_reading_form_token_filter_from_slice_ipadic() {
        use crate::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;

        let config_str = r#"
            {
                "kind": "ipadic"
            }
            "#;
        let result = JapaneseReadingFormTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_reading_form_token_filter_from_slice_unidic() {
        use crate::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;

        let config_str = r#"
            {
                "kind": "unidic"
            }
            "#;
        let result = JapaneseReadingFormTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_reading_form_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token::Token;
        use crate::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "ipadic"
            }
            "#;
        let filter = JapaneseReadingFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田空港"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId(321702, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("羽田空港"),
                    Cow::Borrowed("ハネダクウコウ"),
                    Cow::Borrowed("ハネダクーコー"),
                ]),
            },
            Token {
                text: Cow::Borrowed("限定"),
                byte_start: 12,
                byte_end: 18,
                position: 1,
                position_length: 1,
                word_id: WordId(374175, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("サ変接続"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("限定"),
                    Cow::Borrowed("ゲンテイ"),
                    Cow::Borrowed("ゲンテイ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("トートバッグ"),
                byte_start: 18,
                byte_end: 36,
                position: 2,
                position_length: 1,
                word_id: WordId(4294967295, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![Cow::Borrowed("UNK")]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "ハネダクウコウ");
        assert_eq!(&tokens[1].text, "ゲンテイ");
        assert_eq!(&tokens[2].text, "トートバッグ");
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_reading_form_token_filter_apply_unidic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token::Token;
        use crate::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "unidic"
            }
            "#;
        let filter = JapaneseReadingFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId(618177, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("人名"),
                    Cow::Borrowed("姓"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("羽田"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("羽田"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("固"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("空港"),
                byte_start: 6,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId(587348, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("クウコウ"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("クーコー"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("クーコー"),
                    Cow::Borrowed("漢"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("限定"),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                word_id: WordId(720499, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("サ変可能"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("ゲンテイ"),
                    Cow::Borrowed("限定"),
                    Cow::Borrowed("限定"),
                    Cow::Borrowed("ゲンテー"),
                    Cow::Borrowed("限定"),
                    Cow::Borrowed("ゲンテー"),
                    Cow::Borrowed("漢"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("トート"),
                byte_start: 18,
                byte_end: 27,
                position: 3,
                position_length: 1,
                word_id: WordId(216230, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("外"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("バッグ"),
                byte_start: 27,
                byte_end: 36,
                position: 4,
                position_length: 1,
                word_id: WordId(223781, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("バッグ-bag"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("外"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 5);
        assert_eq!(&tokens[0].text, "ハタ");
        assert_eq!(&tokens[1].text, "クウコウ");
        assert_eq!(&tokens[2].text, "ゲンテイ");
        assert_eq!(&tokens[3].text, "トート");
        assert_eq!(&tokens[4].text, "バッグ");
    }
}
