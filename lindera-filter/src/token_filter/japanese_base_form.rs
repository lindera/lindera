use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_dictionary::DictionaryKind;
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

pub const JAPANESE_BASE_FORM_TOKEN_FILTER_NAME: &str = "japanese_base_form";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseBaseFormTokenFilterConfig {
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

/// Replace the term text with the base form registered in the morphological dictionary.
/// This acts as a lemmatizer for verbs and adjectives.
///
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
    fn name(&self) -> &'static str {
        JAPANESE_BASE_FORM_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if let Some(details) = token.get_details() {
                if details[0] == "UNK" {
                    // NOOP
                    continue;
                }
                match self.config.kind {
                    #[cfg(feature = "ipadic")]
                    DictionaryKind::IPADIC => {
                        token.text = details[6].to_string().into();
                    }
                    #[cfg(feature = "ipadic-neologd")]
                    DictionaryKind::IPADICNEologd => {
                        token.text = details[6].to_string().into();
                    }
                    #[cfg(feature = "unidic")]
                    DictionaryKind::UniDic => {
                        token.text = details[10].to_string().into();
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
        japanese_base_form::{JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig},
        TokenFilter,
    };

    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
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

    #[cfg(all(feature = "unidic", feature = "unidic-filter",))]
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

    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
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

    #[cfg(all(feature = "unidic", feature = "unidic-filter",))]
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

    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    #[test]
    fn test_japanese_base_form_token_filter_apply_ipadic() {
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
        let filter = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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
            Token::new("に", 12, 15, 1, WordId(53041, true), &dictionary, None),
            Token::new("あり", 15, 21, 2, WordId(3222, true), &dictionary, None),
            Token::new("ます", 21, 27, 3, WordId(68730, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].text, "羽田空港".to_string());
        assert_eq!(tokens[1].text, "に".to_string());
        assert_eq!(tokens[2].text, "ある".to_string());
        assert_eq!(tokens[3].text, "ます".to_string());
    }

    #[cfg(all(feature = "unidic", feature = "unidic-filter",))]
    #[test]
    fn test_japanese_base_form_token_filter_apply_unidic() {
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
        let filter = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("羽田", 0, 6, 0, WordId(618177, true), &dictionary, None),
            Token::new("空港", 6, 12, 1, WordId(587348, true), &dictionary, None),
            Token::new("に", 12, 15, 2, WordId(106480, true), &dictionary, None),
            Token::new("あり", 15, 21, 2, WordId(6075, true), &dictionary, None),
            Token::new("ます", 21, 27, 2, WordId(140895, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 5);
        assert_eq!(&tokens[0].text, "羽田");
        assert_eq!(&tokens[1].text, "空港");
        assert_eq!(&tokens[2].text, "に");
        assert_eq!(&tokens[3].text, "ある");
        assert_eq!(&tokens[4].text, "ます");
    }
}
