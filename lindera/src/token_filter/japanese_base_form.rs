use std::borrow::Cow;

use serde_json::Value;

use crate::LinderaResult;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const JAPANESE_BASE_FORM_TOKEN_FILTER_NAME: &str = "japanese_base_form";

pub type JapaneseBaseFormTokenFilterConfig = Value;

/// Replace the term text with the base form registered in the morphological dictionary.
/// This acts as a lemmatizer for verbs and adjectives.
///
#[derive(Clone, Debug)]
pub struct JapaneseBaseFormTokenFilter {}

impl JapaneseBaseFormTokenFilter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn from_config(_config: &JapaneseBaseFormTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self::new())
    }
}

impl Default for JapaneseBaseFormTokenFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFilter for JapaneseBaseFormTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_BASE_FORM_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            // Skip tokens with "UNK" in the first detail
            if let Some(detail) = token.get_detail(0) {
                if detail == "UNK" {
                    continue;
                }
            }

            if let Some(detail) = token.get("base_form") {
                token.text = Cow::Owned(detail.to_string());
            }
            if let Some(detail) = token.get("orthographic_base_form") {
                token.text = Cow::Owned(detail.to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "ipadic", feature = "embedded-ipadic"))]
    #[test]
    fn test_japanese_base_form_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_base_form::JapaneseBaseFormTokenFilter;

        let filter = JapaneseBaseFormTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

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

    #[cfg(all(feature = "unidic", feature = "embedded-unidic"))]
    #[test]
    fn test_japanese_base_form_token_filter_apply_unidic() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_base_form::JapaneseBaseFormTokenFilter;

        let filter = JapaneseBaseFormTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::UniDic).unwrap();

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
