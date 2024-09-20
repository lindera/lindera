use std::borrow::Cow;

use kanaria::string::UCSStr;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const JAPANESE_KANA_TOKEN_FILTER_NAME: &str = "japanese_kana";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum KanaKind {
    /// Katakana to Hiragana.
    #[serde(rename = "hiragana")]
    Hiragana,
    /// Hiragana to Katakana.
    #[serde(rename = "katakana")]
    Katakana,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseKanaTokenFilterConfig {
    kind: KanaKind,
}

impl JapaneseKanaTokenFilterConfig {
    pub fn new(kind: KanaKind) -> Self {
        Self { kind }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<JapaneseKanaTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        serde_json::from_value::<JapaneseKanaTokenFilterConfig>(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Convert only katakana to hiragana, or only hiragana to katakana,
/// that using the specified normalization form, one of 'hiragana' (hiragana to katakana) or 'katakana' (katakana to hiragana).
///
#[derive(Clone, Debug)]
pub struct JapaneseKanaTokenFilter {
    config: JapaneseKanaTokenFilterConfig,
}

impl JapaneseKanaTokenFilter {
    pub fn new(config: JapaneseKanaTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(JapaneseKanaTokenFilterConfig::from_slice(data)?))
    }
}

impl TokenFilter for JapaneseKanaTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_KANA_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            match self.config.kind {
                KanaKind::Hiragana => {
                    // Convert katakana to hiragana.
                    token.text = Cow::Owned(UCSStr::from_str(&token.text).hiragana().to_string());
                }
                KanaKind::Katakana => {
                    // Convert hiragana to katakana.
                    token.text = Cow::Owned(UCSStr::from_str(&token.text).katakana().to_string());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    fn test_japanese_kana_token_filter_config_from_slice_hiragana() {
        use crate::token_filter::japanese_kana::{JapaneseKanaTokenFilterConfig, KanaKind};

        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let config = JapaneseKanaTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, KanaKind::Hiragana);
    }

    #[test]
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    fn test_japanese_kana_token_filter_config_from_slice_katakana() {
        use crate::token_filter::japanese_kana::{JapaneseKanaTokenFilterConfig, KanaKind};

        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let config = JapaneseKanaTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, KanaKind::Katakana);
    }

    #[test]
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    fn test_japanese_kana_token_filter_from_slice_hiragana() {
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilter;

        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let result = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    fn test_japanese_kana_token_filter_from_slice_katakana() {
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilter;

        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let result = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_katakana_to_hiragana_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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
        assert_eq!(&tokens[0].text, "羽田空港");
        assert_eq!(&tokens[1].text, "限定");
        assert_eq!(&tokens[2].text, "とーとばっぐ");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_hiragana_to_katakana_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("埼玉"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId(171030, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("埼玉"),
                    Cow::Borrowed("サイタマ"),
                    Cow::Borrowed("サイタマ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("県"),
                byte_start: 6,
                byte_end: 9,
                position: 1,
                position_length: 1,
                word_id: WordId(298064, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("県"),
                    Cow::Borrowed("ケン"),
                    Cow::Borrowed("ケン"),
                ]),
            },
            Token {
                text: Cow::Borrowed("さいたま"),
                byte_start: 9,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId(28502, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("さいたま"),
                    Cow::Borrowed("サイタマ"),
                    Cow::Borrowed("サイタマ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("市"),
                byte_start: 21,
                byte_end: 24,
                position: 3,
                position_length: 1,
                word_id: WordId(202045, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("市"),
                    Cow::Borrowed("シ"),
                    Cow::Borrowed("シ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "埼玉");
        assert_eq!(&tokens[1].text, "県");
        assert_eq!(&tokens[2].text, "サイタマ");
        assert_eq!(&tokens[3].text, "市");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_katakana_to_katakana_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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
        assert_eq!(&tokens[0].text, "羽田空港");
        assert_eq!(&tokens[1].text, "限定");
        assert_eq!(&tokens[2].text, "トートバッグ");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_hiragana_to_hiragana_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("埼玉"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId(171030, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("埼玉"),
                    Cow::Borrowed("サイタマ"),
                    Cow::Borrowed("サイタマ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("県"),
                byte_start: 6,
                byte_end: 9,
                position: 1,
                position_length: 1,
                word_id: WordId(298064, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("県"),
                    Cow::Borrowed("ケン"),
                    Cow::Borrowed("ケン"),
                ]),
            },
            Token {
                text: Cow::Borrowed("さいたま"),
                byte_start: 9,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId(28502, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("さいたま"),
                    Cow::Borrowed("サイタマ"),
                    Cow::Borrowed("サイタマ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("市"),
                byte_start: 21,
                byte_end: 24,
                position: 3,
                position_length: 1,
                word_id: WordId(202045, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("市"),
                    Cow::Borrowed("シ"),
                    Cow::Borrowed("シ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "埼玉");
        assert_eq!(&tokens[1].text, "県");
        assert_eq!(&tokens[2].text, "さいたま");
        assert_eq!(&tokens[3].text, "市");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_mixed_to_katakana_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("東京"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId(250023, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("東京"),
                    Cow::Borrowed("トウキョウ"),
                    Cow::Borrowed("トーキョー"),
                ]),
            },
            Token {
                text: Cow::Borrowed("都"),
                byte_start: 6,
                byte_end: 9,
                position: 1,
                position_length: 1,
                word_id: WordId(364736, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("都"),
                    Cow::Borrowed("ト"),
                    Cow::Borrowed("ト"),
                ]),
            },
            Token {
                text: Cow::Borrowed("あきる野"),
                byte_start: 9,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId(927, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("あきる野"),
                    Cow::Borrowed("アキルノ"),
                    Cow::Borrowed("アキルノ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("市"),
                byte_start: 21,
                byte_end: 24,
                position: 3,
                position_length: 1,
                word_id: WordId(202045, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("市"),
                    Cow::Borrowed("シ"),
                    Cow::Borrowed("シ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "東京");
        assert_eq!(&tokens[1].text, "都");
        assert_eq!(&tokens[2].text, "アキル野");
        assert_eq!(&tokens[3].text, "市");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_applymixed_to_hiragana_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("南北線"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId(151151, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("南北線"),
                    Cow::Borrowed("ナンボクセン"),
                    Cow::Borrowed("ナンボクセン"),
                ]),
            },
            Token {
                text: Cow::Borrowed("四ツ谷"),
                byte_start: 9,
                byte_end: 18,
                position: 1,
                position_length: 1,
                word_id: WordId(166998, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("四ツ谷"),
                    Cow::Borrowed("ヨツヤ"),
                    Cow::Borrowed("ヨツヤ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("駅"),
                byte_start: 18,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId(383791, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("駅"),
                    Cow::Borrowed("エキ"),
                    Cow::Borrowed("エキ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "南北線");
        assert_eq!(&tokens[1].text, "四つ谷");
        assert_eq!(&tokens[2].text, "駅");
    }
}
