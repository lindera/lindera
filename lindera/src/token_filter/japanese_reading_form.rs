#[cfg(any(feature = "ipadic", feature = "unidic",))]
use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, DictionaryKind, LinderaResult, Token};

pub const JAPANESE_READING_FORM_TOKEN_FILTER_NAME: &str = "japanese_reading_form";

fn default_kind() -> DictionaryKind {
    DictionaryKind::IPADIC
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseReadingFormTokenFilterConfig {
    #[serde(default = "default_kind")]
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
}

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
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if let Some(details) = &token.details {
                if &details[0] == "UNK" {
                    // NOOP
                    continue;
                }
                match self.config.kind {
                    #[cfg(feature = "ipadic")]
                    DictionaryKind::IPADIC => {
                        token.text = Cow::Owned(details[7].clone());
                    }
                    #[cfg(feature = "unidic")]
                    DictionaryKind::UniDic => {
                        token.text = Cow::Owned(details[6].clone());
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "ipadic", feature = "unidic",))]
    use std::borrow::Cow;

    #[cfg(any(feature = "ipadic", feature = "unidic",))]
    use lindera_core::token_filter::TokenFilter;

    #[cfg(any(feature = "ipadic", feature = "unidic",))]
    use crate::{
        token_filter::japanese_reading_form::{
            JapaneseReadingFormTokenFilter, JapaneseReadingFormTokenFilterConfig,
        },
        DictionaryKind, Token,
    };

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_reading_form_token_filter_config_from_slice_ipadic() {
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
        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let filter = JapaneseReadingFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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
            },
            Token {
                text: Cow::Borrowed("トートバッグ"),
                details: Some(vec!["UNK".to_string()]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "ハネダクウコウ");
        assert_eq!(tokens[1].text, "ゲンテイ");
        assert_eq!(tokens[2].text, "トートバッグ");
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_reading_form_token_filter_apply_unidic() {
        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let filter = JapaneseReadingFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田"),
                details: Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "人名".to_string(),
                    "姓".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "ハタ".to_string(),
                    "ハタ".to_string(),
                    "羽田".to_string(),
                    "ハタ".to_string(),
                    "羽田".to_string(),
                    "ハタ".to_string(),
                    "固".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("空港"),
                details: Some(vec![
                    "名詞".to_string(),
                    "普通名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "クウコウ".to_string(),
                    "空港".to_string(),
                    "空港".to_string(),
                    "クーコー".to_string(),
                    "空港".to_string(),
                    "クーコー".to_string(),
                    "漢".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("限定"),
                details: Some(vec![
                    "名詞".to_string(),
                    "普通名詞".to_string(),
                    "サ変可能".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "ゲンテイ".to_string(),
                    "限定".to_string(),
                    "限定".to_string(),
                    "ゲンテー".to_string(),
                    "限定".to_string(),
                    "ゲンテー".to_string(),
                    "漢".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("トート"),
                details: Some(vec![
                    "名詞".to_string(),
                    "普通名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "トート".to_string(),
                    "トート".to_string(),
                    "トート".to_string(),
                    "トート".to_string(),
                    "トート".to_string(),
                    "トート".to_string(),
                    "外".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string()]),
            },
            Token {
                text: Cow::Borrowed("バッグ"),
                details: Some(vec![
                    "名詞".to_string(),
                    "普通名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "バッグ".to_string(),
                    "バッグ-bag".to_string(),
                    "バッグ".to_string(),
                    "バッグ".to_string(),
                    "バッグ".to_string(),
                    "バッグ".to_string(),
                    "外".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string()]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].text, "ハタ");
        assert_eq!(tokens[1].text, "クウコウ");
        assert_eq!(tokens[2].text, "ゲンテイ");
        assert_eq!(tokens[3].text, "トート");
        assert_eq!(tokens[4].text, "バッグ");
    }
}
