use std::borrow::Cow;
use std::str::FromStr;

use kanaria::string::UCSStr;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{LinderaError, LinderaErrorKind};
use crate::token::Token;
use crate::token_filter::TokenFilter;
use crate::LinderaResult;

pub const JAPANESE_KANA_TOKEN_FILTER_NAME: &str = "japanese_kana";

pub type JapaneseKanaTokenFilterConfig = Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum KanaKind {
    /// Katakana to Hiragana.
    #[serde(rename = "hiragana")]
    Hiragana,
    /// Hiragana to Katakana.
    #[serde(rename = "katakana")]
    Katakana,
}

impl KanaKind {
    pub fn as_str(&self) -> &str {
        match self {
            KanaKind::Hiragana => "hiragana",
            KanaKind::Katakana => "katakana",
        }
    }
}

impl FromStr for KanaKind {
    type Err = LinderaError;
    fn from_str(kind: &str) -> Result<Self, Self::Err> {
        match kind {
            "hiragana" => Ok(KanaKind::Hiragana),
            "katakana" => Ok(KanaKind::Katakana),
            _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!("Invalid kana kind"))),
        }
    }
}

/// Convert only katakana to hiragana, or only hiragana to katakana,
/// that using the specified normalization form, one of 'hiragana' (hiragana to katakana) or 'katakana' (katakana to hiragana).
///
#[derive(Clone, Debug)]
pub struct JapaneseKanaTokenFilter {
    // config: JapaneseKanaTokenFilterConfig,
    kind: KanaKind,
}

impl JapaneseKanaTokenFilter {
    pub fn new(kind: KanaKind) -> Self {
        Self { kind }
    }

    pub fn from_config(config: &JapaneseKanaTokenFilterConfig) -> LinderaResult<Self> {
        let kind = config
            .get("kind")
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing kind config."))
            })?
            .as_str()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("invalid kind config."))
            })?;
        let kind = KanaKind::from_str(kind)?;

        Ok(Self::new(kind))
    }
}

impl TokenFilter for JapaneseKanaTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_KANA_TOKEN_FILTER_NAME
    }

    /// Converts the text of each token from katakana to hiragana or vice versa based on the configuration.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. The `text` field of each token will be modified in place.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating whether the operation was successful.
    ///
    /// # Process
    ///
    /// 1. **Token Iteration**:
    ///    - The function iterates over each token in the `tokens` vector.
    ///
    /// 2. **Kana Conversion**:
    ///    - Depending on the configuration (`self.config.kind`):
    ///      - If `KanaKind::Hiragana` is selected, katakana characters in the token's text are converted to hiragana.
    ///      - If `KanaKind::Katakana` is selected, hiragana characters in the token's text are converted to katakana.
    ///
    /// 3. **Text Update**:
    ///    - The converted text is then assigned back to the token's `text` field as `Cow::Owned`.
    ///
    /// # KanaKind:
    ///
    /// - **Hiragana**: Converts any katakana in the token's text to hiragana.
    /// - **Katakana**: Converts any hiragana in the token's text to katakana.
    ///
    /// # Errors
    ///
    /// If any issue arises during the token processing or text conversion, the function will return an error in the form of `LinderaResult`.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            let converted_text = match self.kind {
                KanaKind::Hiragana => {
                    // Convert katakana to hiragana.
                    UCSStr::from_str(&token.text).hiragana().to_string()
                }
                KanaKind::Katakana => {
                    // Convert hiragana to katakana.
                    UCSStr::from_str(&token.text).katakana().to_string()
                }
            };

            token.text = Cow::Owned(converted_text);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    fn test_japanese_kana_token_filter_config_hiragana() {
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilterConfig;

        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let result: Result<JapaneseKanaTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    fn test_japanese_kana_token_filter_config_katakana() {
        use crate::token_filter::japanese_kana::JapaneseKanaTokenFilterConfig;

        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let result: Result<JapaneseKanaTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    fn test_japanese_kana_token_filter_hiragana() {
        use crate::token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig,
        };

        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let config: JapaneseKanaTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = JapaneseKanaTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    fn test_japanese_kana_token_filter_from_slice_katakana() {
        use crate::token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig,
        };

        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let config: JapaneseKanaTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = JapaneseKanaTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_katakana_to_hiragana_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let config: JapaneseKanaTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseKanaTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田空港"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 321702,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 374175,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 4294967295,
                    is_system: true,
                },
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

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let config: JapaneseKanaTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseKanaTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("埼玉"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 171030,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 298064,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 28502,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 202045,
                    is_system: true,
                },
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

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let config: JapaneseKanaTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseKanaTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田空港"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 321702,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 374175,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 4294967295,
                    is_system: true,
                },
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

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let config: JapaneseKanaTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseKanaTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("埼玉"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 171030,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 298064,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 28502,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 202045,
                    is_system: true,
                },
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

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let config: JapaneseKanaTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseKanaTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("東京"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 250023,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 364736,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 927,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 202045,
                    is_system: true,
                },
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

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let config: JapaneseKanaTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseKanaTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("南北線"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 151151,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 166998,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 383791,
                    is_system: true,
                },
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
