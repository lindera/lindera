#[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::dictionary::DictionaryKind;
use crate::token::Token;
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

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        serde_json::from_value::<JapaneseBaseFormTokenFilterConfig>(value.clone())
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
            if let Some(detail) = token.get_detail(0) {
                if detail == "UNK" {
                    continue;
                }
            }

            // Get the index of the detail that contains the base form.
            #[allow(unused_variables)]
            let detail_index = match self.config.kind {
                #[cfg(feature = "ipadic")]
                DictionaryKind::IPADIC => 6,
                #[cfg(feature = "ipadic-neologd")]
                DictionaryKind::IPADICNEologd => 6,
                #[cfg(feature = "unidic")]
                DictionaryKind::UniDic => 10,
                _ => continue,
            };

            // Make token text the base form.
            #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
            if let Some(detail) = token.get_detail(detail_index) {
                token.text = Cow::Owned(detail.to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    use std::borrow::Cow;

    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    use lindera_core::dictionary::word_entry::WordId;

    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    use crate::dictionary::{DictionaryKind, DictionaryLoader};
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    use crate::token::Token;
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    use crate::token_filter::japanese_base_form::{
        JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig,
    };
    #[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
    use crate::token_filter::TokenFilter;

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
    fn test_japanese_base_form_token_filter_apply_ipadic() {
        let config_str = r#"
            {
                "kind": "ipadic"
            }
            "#;
        let filter = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田空港"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId(321702, true),
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
                text: Cow::Borrowed("に"),
                byte_start: 12,
                byte_end: 15,
                position: 1,
                position_length: 1,
                word_id: WordId(53041, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助詞"),
                    Cow::Borrowed("格助詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("に"),
                    Cow::Borrowed("ニ"),
                    Cow::Borrowed("ニ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("あり"),
                byte_start: 15,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId(3222, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("動詞"),
                    Cow::Borrowed("自立"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("五段・ラ行"),
                    Cow::Borrowed("基本形"),
                    Cow::Borrowed("ある"),
                    Cow::Borrowed("アリ"),
                    Cow::Borrowed("アリ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("ます"),
                byte_start: 21,
                byte_end: 27,
                position: 3,
                position_length: 1,
                word_id: WordId(68730, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助動詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("特殊・マス"),
                    Cow::Borrowed("基本形"),
                    Cow::Borrowed("ます"),
                    Cow::Borrowed("マス"),
                    Cow::Borrowed("マス"),
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

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_base_form_token_filter_apply_unidic() {
        let config_str = r#"
            {
                "kind": "unidic"
            }
            "#;
        let filter = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId(618177, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("人名"),
                    Cow::Borrowed("姓"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("羽田"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("羽田"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("固"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("空港"),
                byte_start: 6,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId(587348, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("クウコウ"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("クーコー"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("クーコー"),
                    Cow::Borrowed("漢"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("に"),
                byte_start: 12,
                byte_end: 15,
                position: 2,
                position_length: 1,
                word_id: WordId(106480, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助詞"),
                    Cow::Borrowed("格助詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("ニ"),
                    Cow::Borrowed("に"),
                    Cow::Borrowed("に"),
                    Cow::Borrowed("ニ"),
                    Cow::Borrowed("に"),
                    Cow::Borrowed("ニ"),
                    Cow::Borrowed("和"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("あり"),
                byte_start: 15,
                byte_end: 21,
                position: 3,
                position_length: 1,
                word_id: WordId(6075, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("動詞"),
                    Cow::Borrowed("非自立可能"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("五段-ラ行"),
                    Cow::Borrowed("連用形-一般"),
                    Cow::Borrowed("アル"),
                    Cow::Borrowed("有る"),
                    Cow::Borrowed("あり"),
                    Cow::Borrowed("アリ"),
                    Cow::Borrowed("ある"),
                    Cow::Borrowed("アル"),
                    Cow::Borrowed("和"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("ます"),
                byte_start: 21,
                byte_end: 27,
                position: 4,
                position_length: 1,
                word_id: WordId(140895, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助動詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("助動詞-マス"),
                    Cow::Borrowed("終止形-一般"),
                    Cow::Borrowed("マス"),
                    Cow::Borrowed("ます"),
                    Cow::Borrowed("ます"),
                    Cow::Borrowed("マス"),
                    Cow::Borrowed("ます"),
                    Cow::Borrowed("マス"),
                    Cow::Borrowed("和"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
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
