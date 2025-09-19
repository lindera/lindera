use std::borrow::Cow;

use serde_json::Value;

use crate::LinderaResult;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const LOWERCASE_TOKEN_FILTER_NAME: &str = "lowercase";

pub type LowercaseTokenFilterConfig = Value;

/// Normalizes token text to lowercase.
///
#[derive(Clone, Debug)]
pub struct LowercaseTokenFilter {}

impl LowercaseTokenFilter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn from_config(_config: &LowercaseTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self::new())
    }
}

impl Default for LowercaseTokenFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFilter for LowercaseTokenFilter {
    fn name(&self) -> &'static str {
        LOWERCASE_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.surface = Cow::Owned(token.surface.to_lowercase());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "embedded-ipadic")]
    fn test_lowercase_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_dictionary::viterbi::LexType;
        use crate::token::Token;
        use crate::token_filter::TokenFilter;
        use crate::token_filter::lowercase::LowercaseTokenFilter;

        let filter = LowercaseTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![Token {
            surface: Cow::Borrowed("Rust"),
            byte_start: 0,
            byte_end: 4,
            position: 0,
            position_length: 1,
            word_id: WordId {
                id: 4294967295,
                is_system: true,
                lex_type: LexType::System,
            },
            dictionary: &dictionary,
            user_dictionary: None,
            details: Some(vec![Cow::Borrowed("UNK")]),
        }];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].surface, "rust");
    }
}
