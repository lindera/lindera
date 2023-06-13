use kanaria::string::UCSStr;
use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_tokenizer::token::Token;

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
                    token.text = UCSStr::from_str(&token.text).hiragana().to_string().into();
                }
                KanaKind::Katakana => {
                    token.text = UCSStr::from_str(&token.text).katakana().to_string().into();
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use lindera_core::word_entry::WordId;
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use lindera_tokenizer::token::Token;

    use crate::token_filter::japanese_kana::{
        JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig, KanaKind,
    };
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use crate::token_filter::TokenFilter;

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
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    fn test_japanese_kana_token_filter_apply_katakana_to_hiragana_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new(
                "羽田空港",
                0,
                12,
                0,
                WordId(321702, true),
                &dictionary,
                None,
            ),
            Token::new("限定", 12, 18, 1, WordId(374175, true), &dictionary, None),
            Token::new(
                "トートバッグ",
                18,
                36,
                2,
                WordId(4294967295, true),
                &dictionary,
                None,
            ),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "羽田空港");
        assert_eq!(&tokens[1].text, "限定");
        assert_eq!(&tokens[2].text, "とーとばっぐ");
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    fn test_japanese_kana_token_filter_apply_hiragana_to_katakana_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("埼玉", 0, 6, 0, WordId(171030, true), &dictionary, None),
            Token::new("県", 6, 9, 1, WordId(298064, true), &dictionary, None),
            Token::new("さいたま", 9, 21, 2, WordId(28502, true), &dictionary, None),
            Token::new("市", 21, 24, 3, WordId(202045, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "埼玉");
        assert_eq!(&tokens[1].text, "県");
        assert_eq!(&tokens[2].text, "サイタマ");
        assert_eq!(&tokens[3].text, "市");
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    fn test_japanese_kana_token_filter_apply_katakana_to_katakana_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new(
                "羽田空港",
                0,
                12,
                0,
                WordId(321702, true),
                &dictionary,
                None,
            ),
            Token::new("限定", 12, 18, 1, WordId(374175, true), &dictionary, None),
            Token::new(
                "トートバッグ",
                18,
                36,
                2,
                WordId(4294967295, true),
                &dictionary,
                None,
            ),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "羽田空港");
        assert_eq!(&tokens[1].text, "限定");
        assert_eq!(&tokens[2].text, "トートバッグ");
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    fn test_japanese_kana_token_filter_apply_hiragana_to_hiragana_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("埼玉", 0, 6, 0, WordId(171030, true), &dictionary, None),
            Token::new("県", 6, 9, 1, WordId(298064, true), &dictionary, None),
            Token::new("さいたま", 9, 21, 2, WordId(28502, true), &dictionary, None),
            Token::new("市", 21, 24, 3, WordId(202045, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "埼玉");
        assert_eq!(&tokens[1].text, "県");
        assert_eq!(&tokens[2].text, "さいたま");
        assert_eq!(&tokens[3].text, "市");
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    fn test_japanese_kana_token_filter_apply_mixed_to_katakana_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "katakana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("東京", 0, 6, 0, WordId(250023, true), &dictionary, None),
            Token::new("都", 6, 9, 1, WordId(364736, true), &dictionary, None),
            Token::new("あきる野", 9, 21, 2, WordId(927, true), &dictionary, None),
            Token::new("市", 21, 24, 3, WordId(202045, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "東京");
        assert_eq!(&tokens[1].text, "都");
        assert_eq!(&tokens[2].text, "アキル野");
        assert_eq!(&tokens[3].text, "市");
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    fn test_japanese_kana_token_filter_applymixed_to_hiragana_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "hiragana"
            }
            "#;
        let filter = JapaneseKanaTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("南北線", 0, 9, 0, WordId(151151, true), &dictionary, None),
            Token::new("四ツ谷", 9, 18, 1, WordId(166998, true), &dictionary, None),
            Token::new("駅", 18, 21, 2, WordId(383791, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "南北線");
        assert_eq!(&tokens[1].text, "四つ谷");
        assert_eq!(&tokens[2].text, "駅");
    }
}
