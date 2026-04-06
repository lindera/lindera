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
            if let Some(pos) = token.get_detail(0)
                && pos == "UNK"
            {
                continue;
            }

            if let Some(base_form) = token.get("base_form") {
                token.surface = Cow::Owned(base_form.to_string());
            }
            if let Some(base_form) = token.get("orthographic_base_form") {
                token.surface = Cow::Owned(base_form.to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_base_form::JapaneseBaseFormTokenFilter;
        use lindera_dictionary::viterbi::LexType;

        let filter = JapaneseBaseFormTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                surface: Cow::Borrowed("羽田空港"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 321702,
                    is_system: true,
                    lex_type: LexType::System,
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
                surface: Cow::Borrowed("に"),
                byte_start: 12,
                byte_end: 15,
                position: 1,
                position_length: 1,
                word_id: WordId {
                    id: 53041,
                    is_system: true,
                    lex_type: LexType::System,
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
                surface: Cow::Borrowed("あり"),
                byte_start: 15,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId {
                    id: 3222,
                    is_system: true,
                    lex_type: LexType::System,
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
                surface: Cow::Borrowed("ます"),
                byte_start: 21,
                byte_end: 27,
                position: 3,
                position_length: 1,
                word_id: WordId {
                    id: 68730,
                    is_system: true,
                    lex_type: LexType::System,
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
        assert_eq!(tokens[0].surface, "羽田空港");
        assert_eq!(tokens[1].surface, "に");
        assert_eq!(tokens[2].surface, "ある");
        assert_eq!(tokens[3].surface, "ます");
    }
}
