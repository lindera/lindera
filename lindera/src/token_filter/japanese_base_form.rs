#[cfg(any(feature = "ipadic", feature = "ipadic-neologd", feature = "unidic",))]
use std::borrow::Cow;
use std::str::FromStr;

use serde_json::Value;

use crate::dictionary::DictionaryKind;
use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;
use crate::LinderaResult;

pub const JAPANESE_BASE_FORM_TOKEN_FILTER_NAME: &str = "japanese_base_form";

pub type JapaneseBaseFormTokenFilterConfig = Value;

/// Replace the term text with the base form registered in the morphological dictionary.
/// This acts as a lemmatizer for verbs and adjectives.
///
#[derive(Clone, Debug)]
pub struct JapaneseBaseFormTokenFilter {
    kind: DictionaryKind,
}

impl JapaneseBaseFormTokenFilter {
    pub fn new(kind: DictionaryKind) -> Self {
        Self { kind }
    }

    pub fn from_config(config: &JapaneseBaseFormTokenFilterConfig) -> LinderaResult<Self> {
        let kind = config
            .get("kind")
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing kind config."))
            })?
            .as_str()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("invalid kind config."))
            })?;
        let kind = DictionaryKind::from_str(kind)?;

        Ok(Self::new(kind))
    }
}

impl TokenFilter for JapaneseBaseFormTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_BASE_FORM_TOKEN_FILTER_NAME
    }

    /// Applies the base form normalization to tokens based on the dictionary type.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. Each token's `text` will be modified to its base form according to the specified dictionary.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating whether the operation was successful.
    ///
    /// # Process
    ///
    /// 1. **Token Filtering**:
    ///    - For each token, if the first detail (`detail[0]`) is `"UNK"` (unknown), the token is skipped, as it does not require normalization.
    ///
    /// 2. **Base Form Extraction**:
    ///    - Depending on the configured dictionary type (`IPADIC`, `IPADICNeologd`, or `UniDic`), the function determines which detail contains the base form of the token.
    ///    - For `IPADIC` and `IPADICNeologd`, the base form is located at `detail[6]`.
    ///    - For `UniDic`, the base form is located at `detail[10]`.
    ///
    /// 3. **Text Modification**:
    ///    - Once the correct base form detail is identified, the token's `text` is replaced with the base form using `Cow::Owned`.
    ///
    /// # Dictionary Types:
    ///
    /// - **IPADIC**: Uses index `6` for base form extraction.
    /// - **IPADICNeologd**: Also uses index `6` for base form extraction.
    /// - **UniDic**: Uses index `10` for base form extraction.
    ///
    /// # Errors
    ///
    /// If any issue arises while processing tokens, the function will return an error in the form of `LinderaResult`.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            if let Some(detail) = token.get_detail(0) {
                if detail == "UNK" {
                    continue;
                }
            }

            // Get the index of the detail that contains the base form.
            match self.kind {
                #[cfg(feature = "ipadic")]
                DictionaryKind::IPADIC => {
                    if let Some(detail) = token.get_detail(6) {
                        token.text = Cow::Owned(detail.to_string());
                    }
                }
                #[cfg(feature = "ipadic-neologd")]
                DictionaryKind::IPADICNEologd => {
                    if let Some(detail) = token.get_detail(6) {
                        token.text = Cow::Owned(detail.to_string());
                    }
                }
                #[cfg(feature = "unidic")]
                DictionaryKind::UniDic => {
                    if let Some(detail) = token.get_detail(10) {
                        token.text = Cow::Owned(detail.to_string());
                    }
                }
                _ => continue,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_japanese_base_form_token_filter_config_ipadic() {
        use crate::token_filter::japanese_base_form::JapaneseBaseFormTokenFilterConfig;

        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let result: Result<JapaneseBaseFormTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_japanese_base_form_token_filter_config_funidic() {
        use crate::token_filter::japanese_base_form::JapaneseBaseFormTokenFilterConfig;

        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let result: Result<JapaneseBaseFormTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_japanese_base_form_token_filter_ipadic() {
        use crate::token_filter::japanese_base_form::{
            JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig,
        };

        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let config: JapaneseBaseFormTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = JapaneseBaseFormTokenFilter::from_config(&config);

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_japanese_base_form_token_filter_from_slice_unidic() {
        use crate::token_filter::japanese_base_form::{
            JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig,
        };

        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let config: JapaneseBaseFormTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = JapaneseBaseFormTokenFilter::from_config(&config);

        assert_eq!(true, result.is_ok());
    }

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_base_form::{
            JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let config: JapaneseBaseFormTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseBaseFormTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田空港"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 321702,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 53041,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 3222,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 68730,
                    is_system: true,
                },
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
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_base_form::{
            JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let config: JapaneseBaseFormTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseBaseFormTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 618177,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 587348,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 106480,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 6075,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 140895,
                    is_system: true,
                },
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
