use std::borrow::Cow;

use crate::core::LinderaResult;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const UPPERCASE_TOKEN_FILTER_NAME: &str = "uppercase";

/// Normalizes token text to uppercase.
///
#[derive(Clone, Debug)]
pub struct UppercaseTokenFilter {}

impl UppercaseTokenFilter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for UppercaseTokenFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFilter for UppercaseTokenFilter {
    fn name(&self) -> &'static str {
        UPPERCASE_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.text = Cow::Owned(token.text.to_uppercase());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "cc-cedict",
        feature = "ko-dic"
    ))]
    use std::borrow::Cow;

    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "cc-cedict",
        feature = "ko-dic"
    ))]
    use lindera_core::dictionary::word_entry::WordId;

    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "cc-cedict",
        feature = "ko-dic"
    ))]
    use crate::dictionary::{DictionaryKind, DictionaryLoader};
    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "cc-cedict",
        feature = "ko-dic"
    ))]
    use crate::token::Token;
    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "cc-cedict",
        feature = "ko-dic"
    ))]
    use crate::token_filter::uppercase::UppercaseTokenFilter;
    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "cc-cedict",
        feature = "ko-dic"
    ))]
    use crate::token_filter::TokenFilter;

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_uppercase_token_filter_apply() {
        let filter = UppercaseTokenFilter::default();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![Token {
            text: Cow::Borrowed("Rust"),
            byte_start: 0,
            byte_end: 4,
            position: 0,
            position_length: 1,
            word_id: WordId(4294967295, true),
            dictionary: &dictionary,
            user_dictionary: None,
            details: Some(vec![Cow::Borrowed("UNK")]),
        }];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].text, "RUST");
    }
}
