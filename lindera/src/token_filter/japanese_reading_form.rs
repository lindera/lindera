use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, DictionaryKind, LinderaResult, Token};

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
            let details = match token.get_details() {
                Some(details) => details,
                None => {
                    return Err(LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!("Failed to get word details.")))
                }
            };

            if details[0] == "UNK" {
                // NOOP
                continue;
            }
            match self.config.kind {
                #[cfg(feature = "ipadic")]
                DictionaryKind::IPADIC => {
                    let new_text = details[7].to_string();
                    token.set_text(new_text);
                }
                #[cfg(feature = "unidic")]
                DictionaryKind::UniDic => {
                    let new_text = details[6].to_string();
                    token.set_text(new_text);
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
    #[cfg(any(feature = "ipadic", feature = "unidic",))]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    #[cfg(any(feature = "ipadic", feature = "unidic",))]
    use crate::{
        builder,
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

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("羽田空港", 0, 12, 0, WordId::default(), &dictionary, None)
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
            Token::new("限定", 12, 18, 1, WordId::default(), &dictionary, None)
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
            Token::new(
                "トートバッグ",
                18,
                36,
                2,
                WordId::default(),
                &dictionary,
                None,
            )
            .set_details(Some(vec!["UNK".to_string()]))
            .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].get_text(), "ハネダクウコウ");
        assert_eq!(tokens[1].get_text(), "ゲンテイ");
        assert_eq!(tokens[2].get_text(), "トートバッグ");
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

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("羽田", 0, 6, 0, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
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
                ]))
                .clone(),
            Token::new("空港", 6, 12, 1, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
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
                ]))
                .clone(),
            Token::new("限定", 12, 18, 2, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
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
                ]))
                .clone(),
            Token::new("トート", 18, 27, 3, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
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
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("バッグ", 27, 36, 4, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
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
                    "*".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].get_text(), "ハタ");
        assert_eq!(tokens[1].get_text(), "クウコウ");
        assert_eq!(tokens[2].get_text(), "ゲンテイ");
        assert_eq!(tokens[3].get_text(), "トート");
        assert_eq!(tokens[4].get_text(), "バッグ");
    }
}
