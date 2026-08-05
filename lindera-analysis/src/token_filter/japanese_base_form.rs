use std::borrow::Cow;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera::LinderaResult;
use lindera::token::Token;

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
            // Skip unknown tokens. Their details come from `unk.def`, so the
            // first one is a real part of speech rather than the `UNK` sentinel
            // this used to test for.
            if token.word_id.is_unknown() {
                continue;
            }

            // `*` is the MeCab placeholder for an absent field, and `details` is
            // padded with it to the schema width, so an entry that carries no
            // base form yields `Some("*")` rather than `None`.
            if let Some(base_form) = token.get("base_form")
                && base_form != "*"
            {
                token.surface = Cow::Owned(base_form.to_string());
            }
            if let Some(base_form) = token.get("orthographic_base_form")
                && base_form != "*"
            {
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

        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_base_form::JapaneseBaseFormTokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
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
                word_id: WordId::new(LexType::System, 321702),
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
                word_id: WordId::new(LexType::System, 53041),
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
                word_id: WordId::new(LexType::System, 3222),
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
                word_id: WordId::new(LexType::System, 68730),
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

    // A word that is not in the dictionary takes its details from `unk.def`, so
    // its part of speech is a real one and its base form is the placeholder `*`.
    // Neither may be substituted into the surface form.
    #[cfg(feature = "embed-ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_keeps_surface_without_base_form() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_base_form::JapaneseBaseFormTokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let filter = JapaneseBaseFormTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        // The details an out-of-vocabulary word receives from `unk.def`: a real
        // part of speech, then `*` for every remaining field.
        let unknown_details = vec![
            Cow::Borrowed("名詞"),
            Cow::Borrowed("固有名詞"),
            Cow::Borrowed("組織"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
        ];

        let mut tokens: Vec<Token> = vec![
            // Out of vocabulary, so skipped by the unknown-word guard.
            Token {
                surface: Cow::Borrowed("asci"),
                byte_start: 0,
                byte_end: 4,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::Unknown, 3),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(unknown_details.clone()),
            },
            // In the dictionary, but carries no base form, so skipped by the
            // placeholder guard.
            Token {
                surface: Cow::Borrowed("("),
                byte_start: 4,
                byte_end: 5,
                position: 1,
                position_length: 1,
                word_id: WordId::new(LexType::System, 3),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(unknown_details),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "asci");
        assert_eq!(tokens[1].surface, "(");
    }
}
