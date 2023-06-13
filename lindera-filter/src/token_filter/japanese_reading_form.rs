use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_dictionary::DictionaryKind;
use lindera_tokenizer::token::Token;

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
            if let Some(details) = &mut token.get_details() {
                if details[0] == "UNK" {
                    // NOOP
                    continue;
                }
                match self.config.kind {
                    #[cfg(feature = "ipadic")]
                    DictionaryKind::IPADIC => {
                        token.text = details[7].to_string().into();
                    }
                    #[cfg(feature = "ipadic-neologd")]
                    DictionaryKind::IPADICNEologd => {
                        token.text = details[7].to_string().into();
                    }
                    #[cfg(feature = "unidic")]
                    DictionaryKind::UniDic => {
                        token.text = details[6].to_string().into();
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
    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "unidic", feature = "unidic-filter",)
    ))]
    use lindera_core::word_entry::WordId;
    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "unidic", feature = "unidic-filter",)
    ))]
    use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "unidic", feature = "unidic-filter",)
    ))]
    use lindera_tokenizer::token::Token;

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
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "ipadic"
            }
            "#;
        let filter = JapaneseReadingFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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
        assert_eq!(&tokens[0].text, "ハネダクウコウ");
        assert_eq!(&tokens[1].text, "ゲンテイ");
        assert_eq!(&tokens[2].text, "トートバッグ");
    }

    #[cfg(all(feature = "unidic", feature = "unidic-filter",))]
    #[test]
    fn test_japanese_reading_form_token_filter_apply_unidic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "unidic"
            }
            "#;
        let filter = JapaneseReadingFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("羽田", 0, 6, 0, WordId(618177, true), &dictionary, None),
            Token::new("空港", 6, 12, 1, WordId(587348, true), &dictionary, None),
            Token::new("限定", 12, 18, 2, WordId(720499, true), &dictionary, None),
            Token::new("トート", 18, 27, 3, WordId(216230, true), &dictionary, None),
            Token::new("バッグ", 27, 36, 4, WordId(223781, true), &dictionary, None),
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
