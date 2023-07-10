use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_dictionary::DictionaryKind;

use crate::{token::Token, token_filter::TokenFilter};

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

    fn apply<'a>(&self, tokens: &mut Vec<Token>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if token.details[0] == "UNK" {
                // NOOP
                continue;
            }
            match self.config.kind {
                #[cfg(feature = "ipadic")]
                DictionaryKind::IPADIC => {
                    token.text = token.details[7].to_string().into();
                }
                #[cfg(feature = "ipadic-neologd")]
                DictionaryKind::IPADICNEologd => {
                    token.text = token.details[7].to_string().into();
                }
                #[cfg(feature = "unidic")]
                DictionaryKind::UniDic => {
                    token.text = token.details[6].to_string().into();
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
    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "unidic", feature = "unidic-filter",)
    ))]
    use crate::token::Token;
    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "unidic", feature = "unidic-filter",)
    ))]
    use lindera_core::word_entry::WordId;
    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "unidic", feature = "unidic-filter",)
    ))]
    use lindera_dictionary::DictionaryKind;

    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "unidic", feature = "unidic-filter",)
    ))]
    use crate::token_filter::{
        japanese_reading_form::{
            JapaneseReadingFormTokenFilter, JapaneseReadingFormTokenFilterConfig,
        },
        TokenFilter,
    };

    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
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

    #[cfg(all(feature = "unidic", feature = "unidic-filter",))]
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

    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
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

    #[cfg(all(feature = "unidic", feature = "unidic-filter",))]
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

    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
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
                text: "羽田空港".to_string(),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId(321702, true),
                details: vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "羽田空港".to_string(),
                    "ハネダクウコウ".to_string(),
                    "ハネダクーコー".to_string(),
                ],
            },
            Token {
                text: "限定".to_string(),
                byte_start: 12,
                byte_end: 18,
                position: 1,
                position_length: 1,
                word_id: WordId(374175, true),
                details: vec![
                    "名詞".to_string(),
                    "サ変接続".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "限定".to_string(),
                    "ゲンテイ".to_string(),
                    "ゲンテイ".to_string(),
                ],
            },
            Token {
                text: "トートバッグ".to_string(),
                byte_start: 18,
                byte_end: 36,
                position: 2,
                position_length: 1,
                word_id: WordId(4294967295, true),
                details: vec!["UNK".to_string()],
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "ハネダクウコウ");
        assert_eq!(&tokens[1].text, "ゲンテイ");
        assert_eq!(&tokens[2].text, "トートバッグ");
    }

    #[cfg(all(feature = "unidic", feature = "unidic-filter",))]
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
                text: "羽田".to_string(),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId(618177, true),
                details: vec![
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
                ],
            },
            Token {
                text: "空港".to_string(),
                byte_start: 6,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId(587348, true),
                details: vec![
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
                ],
            },
            Token {
                text: "限定".to_string(),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                word_id: WordId(720499, true),
                details: vec![
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
                ],
            },
            Token {
                text: "トート".to_string(),
                byte_start: 18,
                byte_end: 27,
                position: 3,
                position_length: 1,
                word_id: WordId(216230, true),
                details: vec![
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
                ],
            },
            Token {
                text: "バッグ".to_string(),
                byte_start: 27,
                byte_end: 36,
                position: 4,
                position_length: 1,
                word_id: WordId(223781, true),
                details: vec![
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
                ],
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
