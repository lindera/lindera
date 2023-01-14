use lindera_core::token_filter::TokenFilter;

use crate::{LinderaResult, Token};

pub const LOWERCASE_TOKEN_FILTER_NAME: &str = "lowercase";

/// Normalizes token text to lower case.
/// 
#[derive(Clone, Debug)]
pub struct LowercaseTokenFilter {}

impl LowercaseTokenFilter {
    pub fn new() -> Self {
        Self {}
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

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.set_text(token.get_text().to_lowercase());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    #[cfg(feature = "ipadic")]
    use crate::{builder, token_filter::lowercase::LowercaseTokenFilter, DictionaryKind, Token};

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_lowercase_token_filter_apply_ipadic() {
        let filter = LowercaseTokenFilter::default();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> =
            vec![
                Token::new("Rust", 0, 4, WordId::default(), &dictionary, None)
                    .set_details(Some(vec!["UNK".to_string()]))
                    .clone(),
            ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].get_text(), "rust");
    }
}
