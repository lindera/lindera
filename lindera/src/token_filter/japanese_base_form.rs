#[cfg(any(feature = "ipadic", feature = "unidic",))]
use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, DictionaryKind, LinderaResult, Token};

pub const JAPANESE_BASE_FORM_TOKEN_FILTER_NAME: &str = "japanese_base_form";

fn default_kind() -> DictionaryKind {
    DictionaryKind::IPADIC
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseBaseFormTokenFilterConfig {
    #[serde(default = "default_kind")]
    kind: DictionaryKind,
}

impl JapaneseBaseFormTokenFilterConfig {
    pub fn new(kind: DictionaryKind) -> Self {
        Self { kind }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<JapaneseBaseFormTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone, Debug)]
pub struct JapaneseBaseFormTokenFilter {
    config: JapaneseBaseFormTokenFilterConfig,
}

impl JapaneseBaseFormTokenFilter {
    pub fn new(config: JapaneseBaseFormTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(JapaneseBaseFormTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for JapaneseBaseFormTokenFilter {
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
                        token.text = Cow::Owned(details[6].clone());
                    }
                    #[cfg(feature = "unidic")]
                    DictionaryKind::UniDic => {
                        token.text = Cow::Owned(details[10].clone());
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
        token_filter::japanese_base_form::{
            JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig,
        },
        DictionaryKind, Token,
    };

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_config_from_slice_ipadic() {
        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let config = JapaneseBaseFormTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, DictionaryKind::IPADIC);
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_base_form_token_filter_config_from_slice_unidic() {
        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let config = JapaneseBaseFormTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, DictionaryKind::UniDic);
    }

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_from_slice_ipadic() {
        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let result = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_base_form_token_filter_from_slice_unidic() {
        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let result = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_apply() {
        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let filter = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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
                text: Cow::Borrowed("に"),
                details: Some(vec![
                    "助詞".to_string(),
                    "格助詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "に".to_string(),
                    "ニ".to_string(),
                    "ニ".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("あり"),
                details: Some(vec![
                    "動詞".to_string(),
                    "自立".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "五段・ラ行".to_string(),
                    "連用形".to_string(),
                    "ある".to_string(),
                    "アリ".to_string(),
                    "アリ".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("ます"),
                details: Some(vec![
                    "助動詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "特殊・マス".to_string(),
                    "基本形".to_string(),
                    "ます".to_string(),
                    "マス".to_string(),
                    "マスS".to_string(),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].text, "羽田空港");
        assert_eq!(tokens[1].text, "に");
        assert_eq!(tokens[2].text, "ある");
        assert_eq!(tokens[3].text, "ます");
    }
}
