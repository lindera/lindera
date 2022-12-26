use std::borrow::Cow;

use kanaria::string::UCSStr;
use lindera_core::token_filter::TokenFilter;
use serde::{Deserialize, Serialize};

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const JAPANESE_KANA_TOKEN_FILTER_NAME: &str = "japanese_kana";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum KanaKind {
    #[serde(rename = "hiragana")]
    Hiragana,
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
}

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
                    token.text = Cow::Owned(UCSStr::from_str(&token.text).hiragana().to_string());
                }
                KanaKind::Katakana => {
                    token.text = Cow::Owned(UCSStr::from_str(&token.text).katakana().to_string());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{
        token_filter::japanese_kana::{
            JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig, KanaKind,
        },
        Token,
    };

    #[test]
    fn test_japanese_kana_token_filter_config_from_slice_hiragana() {
        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let config = JapaneseKanaTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, KanaKind::Hiragana);
    }

    #[test]
    fn test_japanese_kana_token_filter_config_from_slice_katakana() {
        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let config = JapaneseKanaTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, KanaKind::Katakana);
    }

    #[test]
    fn test_japanese_kana_token_filter_from_slice_hiragana() {
        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let result = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_japanese_kana_token_filter_from_slice_katakana() {
        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let result = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_japanese_kana_token_filter_apply_hiragana() {
        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田空港"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "羽田空港".to_string(),
                    "ハネダクウコウ".to_string(),
                    "ハネダクーコー".to_string(),
                ]),
                byte_start: 0,
                byte_end: 12,
            },
            Token {
                text: Cow::Borrowed("限定"),
                details: Some(vec![
                    "名詞".to_string(),
                    "サ変接続".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "限定".to_string(),
                    "ゲンテイ".to_string(),
                    "ゲンテイ".to_string(),
                ]),
                byte_start: 12,
                byte_end: 18,
            },
            Token {
                text: Cow::Borrowed("トートバッグ"),
                details: Some(vec!["UNK".to_string()]),
                byte_start: 18,
                byte_end: 36,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "羽田空港");
        assert_eq!(tokens[1].text, "限定");
        assert_eq!(tokens[2].text, "とーとばっぐ");
    }

    #[test]
    fn test_japanese_kana_token_filter_apply_katakana() {
        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("埼玉"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "埼玉".to_string(),
                    "サイタマ".to_string(),
                    "サイタマ".to_string(),
                ]),
                byte_start: 0,
                byte_end: 6,
            },
            Token {
                text: Cow::Borrowed("県"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "県".to_string(),
                    "ケン".to_string(),
                    "ケン".to_string(),
                ]),
                byte_start: 6,
                byte_end: 9,
            },
            Token {
                text: Cow::Borrowed("さいたま"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "さいたま".to_string(),
                    "サイタマ".to_string(),
                    "サイタマ".to_string(),
                ]),
                byte_start: 9,
                byte_end: 21,
            },
            Token {
                text: Cow::Borrowed("市"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "市".to_string(),
                    "シ".to_string(),
                    "シ".to_string(),
                ]),
                byte_start: 21,
                byte_end: 24,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].text, "埼玉");
        assert_eq!(tokens[1].text, "県");
        assert_eq!(tokens[2].text, "サイタマ");
        assert_eq!(tokens[3].text, "市");
    }

    #[test]
    fn test_japanese_kana_token_filter_apply_katakana2() {
        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田空港"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "羽田空港".to_string(),
                    "ハネダクウコウ".to_string(),
                    "ハネダクーコー".to_string(),
                ]),
                byte_start: 0,
                byte_end: 12,
            },
            Token {
                text: Cow::Borrowed("限定"),
                details: Some(vec![
                    "名詞".to_string(),
                    "サ変接続".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "限定".to_string(),
                    "ゲンテイ".to_string(),
                    "ゲンテイ".to_string(),
                ]),
                byte_start: 12,
                byte_end: 18,
            },
            Token {
                text: Cow::Borrowed("トートバッグ"),
                details: Some(vec!["UNK".to_string()]),
                byte_start: 18,
                byte_end: 36,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "羽田空港");
        assert_eq!(tokens[1].text, "限定");
        assert_eq!(tokens[2].text, "トートバッグ");
    }

    #[test]
    fn test_japanese_kana_token_filter_apply_hiragana2() {
        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("埼玉"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "埼玉".to_string(),
                    "サイタマ".to_string(),
                    "サイタマ".to_string(),
                ]),
                byte_start: 0,
                byte_end: 6,
            },
            Token {
                text: Cow::Borrowed("県"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "県".to_string(),
                    "ケン".to_string(),
                    "ケン".to_string(),
                ]),
                byte_start: 6,
                byte_end: 9,
            },
            Token {
                text: Cow::Borrowed("さいたま"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "さいたま".to_string(),
                    "サイタマ".to_string(),
                    "サイタマ".to_string(),
                ]),
                byte_start: 9,
                byte_end: 21,
            },
            Token {
                text: Cow::Borrowed("市"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "市".to_string(),
                    "シ".to_string(),
                    "シ".to_string(),
                ]),
                byte_start: 21,
                byte_end: 24,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].text, "埼玉");
        assert_eq!(tokens[1].text, "県");
        assert_eq!(tokens[2].text, "さいたま");
        assert_eq!(tokens[3].text, "市");
    }

    #[test]
    fn test_japanese_kana_token_filter_apply_hiragana_mixed() {
        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("東京"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "東京".to_string(),
                    "トウキョウ".to_string(),
                    "トーキョー".to_string(),
                ]),
                byte_start: 0,
                byte_end: 6,
            },
            Token {
                text: Cow::Borrowed("都"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "都".to_string(),
                    "ト".to_string(),
                    "ト".to_string(),
                ]),
                byte_start: 6,
                byte_end: 9,
            },
            Token {
                text: Cow::Borrowed("あきる野"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "あきる野".to_string(),
                    "アキルノ".to_string(),
                    "アキルノ".to_string(),
                ]),
                byte_start: 9,
                byte_end: 21,
            },
            Token {
                text: Cow::Borrowed("市"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "市".to_string(),
                    "シ".to_string(),
                    "シ".to_string(),
                ]),
                byte_start: 21,
                byte_end: 24,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].text, "東京");
        assert_eq!(tokens[1].text, "都");
        assert_eq!(tokens[2].text, "アキル野");
        assert_eq!(tokens[3].text, "市");
    }

    #[test]
    fn test_japanese_kana_token_filter_apply_katakana_mixed() {
        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("南北線"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "南北線".to_string(),
                    "ナンボクセン".to_string(),
                    "ナンボクセン".to_string(),
                ]),
                byte_start: 0,
                byte_end: 9,
            },
            Token {
                text: Cow::Borrowed("四ツ谷"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "四ツ谷".to_string(),
                    "ヨツヤ".to_string(),
                    "ヨツヤ".to_string(),
                ]),
                byte_start: 9,
                byte_end: 18,
            },
            Token {
                text: Cow::Borrowed("駅"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "駅".to_string(),
                    "エキ".to_string(),
                    "エキ".to_string(),
                ]),
                byte_start: 18,
                byte_end: 21,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "南北線");
        assert_eq!(tokens[1].text, "四つ谷");
        assert_eq!(tokens[2].text, "駅");
    }
}
