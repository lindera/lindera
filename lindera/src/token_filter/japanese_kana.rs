use kanaria::string::UCSStr;
use lindera_core::token_filter::TokenFilter;
use serde::{Deserialize, Serialize};

use crate::{error::LinderaErrorKind, LinderaResult, Token};

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
}

/// Convert only katakana to hiragana, or only hiragana to katakana.
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
                    token.set_text(UCSStr::from_str(&token.get_text()).hiragana().to_string());
                }
                KanaKind::Katakana => {
                    token.set_text(UCSStr::from_str(&token.get_text()).katakana().to_string());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    use crate::token_filter::japanese_kana::{
        JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig, KanaKind,
    };

    #[cfg(feature = "ipadic")]
    use crate::{builder, DictionaryKind, Token};

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
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_katakana_to_hiragana_ipadic() {
        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("羽田空港", 0, 12, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "羽田空港".to_string(),
                    "ハネダクウコウ".to_string(),
                    "ハネダクーコー".to_string(),
                ]))
                .clone(),
            Token::new("限定", 12, 18, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "サ変接続".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "限定".to_string(),
                    "ゲンテイ".to_string(),
                    "ゲンテイ".to_string(),
                ]))
                .clone(),
            Token::new("トートバッグ", 18, 36, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].get_text(), "羽田空港");
        assert_eq!(tokens[1].get_text(), "限定");
        assert_eq!(tokens[2].get_text(), "とーとばっぐ");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_hiragana_to_katakana_ipadic() {
        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("埼玉", 0, 6, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "埼玉".to_string(),
                    "サイタマ".to_string(),
                    "サイタマ".to_string(),
                ]))
                .clone(),
            Token::new("県", 6, 9, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "県".to_string(),
                    "ケン".to_string(),
                    "ケン".to_string(),
                ]))
                .clone(),
            Token::new("さいたま", 9, 21, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "さいたま".to_string(),
                    "サイタマ".to_string(),
                    "サイタマ".to_string(),
                ]))
                .clone(),
            Token::new("市", 21, 24, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "市".to_string(),
                    "シ".to_string(),
                    "シ".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].get_text(), "埼玉");
        assert_eq!(tokens[1].get_text(), "県");
        assert_eq!(tokens[2].get_text(), "サイタマ");
        assert_eq!(tokens[3].get_text(), "市");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_katakana_to_katakana_ipadic() {
        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("羽田空港", 0, 12, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "羽田空港".to_string(),
                    "ハネダクウコウ".to_string(),
                    "ハネダクーコー".to_string(),
                ]))
                .clone(),
            Token::new("限定", 12, 18, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "サ変接続".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "限定".to_string(),
                    "ゲンテイ".to_string(),
                    "ゲンテイ".to_string(),
                ]))
                .clone(),
            Token::new("トートバッグ", 18, 36, WordId::default(), &dictionary, None)
                .set_details(Some(vec!["UNK".to_string()]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].get_text(), "羽田空港");
        assert_eq!(tokens[1].get_text(), "限定");
        assert_eq!(tokens[2].get_text(), "トートバッグ");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_hiragana_to_hiragana_ipadic() {
        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("埼玉", 0, 6, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "埼玉".to_string(),
                    "サイタマ".to_string(),
                    "サイタマ".to_string(),
                ]))
                .clone(),
            Token::new("県", 6, 9, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "県".to_string(),
                    "ケン".to_string(),
                    "ケン".to_string(),
                ]))
                .clone(),
            Token::new("さいたま", 9, 21, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "さいたま".to_string(),
                    "サイタマ".to_string(),
                    "サイタマ".to_string(),
                ]))
                .clone(),
            Token::new("市", 21, 24, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "市".to_string(),
                    "シ".to_string(),
                    "シ".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].get_text(), "埼玉");
        assert_eq!(tokens[1].get_text(), "県");
        assert_eq!(tokens[2].get_text(), "さいたま");
        assert_eq!(tokens[3].get_text(), "市");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_hiragana_to_katakana2_ipadic() {
        let config_str = r#"
        {
            "kind": "katakana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("東京", 0, 6, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "東京".to_string(),
                    "トウキョウ".to_string(),
                    "トーキョー".to_string(),
                ]))
                .clone(),
            Token::new("都", 6, 9, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "都".to_string(),
                    "ト".to_string(),
                    "ト".to_string(),
                ]))
                .clone(),
            Token::new("あきる野", 9, 21, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "あきる野".to_string(),
                    "アキルノ".to_string(),
                    "アキルノ".to_string(),
                ]))
                .clone(),
            Token::new("市", 21, 24, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "市".to_string(),
                    "シ".to_string(),
                    "シ".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].get_text(), "東京");
        assert_eq!(tokens[1].get_text(), "都");
        assert_eq!(tokens[2].get_text(), "アキル野");
        assert_eq!(tokens[3].get_text(), "市");
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_kana_token_filter_apply_katakana_to_hiragana2_ipadic() {
        let config_str = r#"
        {
            "kind": "hiragana"
        }
        "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("南北線", 0, 9, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "南北線".to_string(),
                    "ナンボクセン".to_string(),
                    "ナンボクセン".to_string(),
                ]))
                .clone(),
            Token::new("四ツ谷", 9, 18, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "地域".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "四ツ谷".to_string(),
                    "ヨツヤ".to_string(),
                    "ヨツヤ".to_string(),
                ]))
                .clone(),
            Token::new("駅", 18, 21, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "地域".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "駅".to_string(),
                    "エキ".to_string(),
                    "エキ".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].get_text(), "南北線");
        assert_eq!(tokens[1].get_text(), "四つ谷");
        assert_eq!(tokens[2].get_text(), "駅");
    }
}
