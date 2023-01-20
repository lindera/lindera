use lindera_core::token_filter::TokenFilter;

use crate::{LinderaResult, Token};

pub const UPPERCASE_TOKEN_FILTER_NAME: &str = "uppercase";

/// Normalizes token text to upper case.
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
            token.set_text(token.get_text().to_uppercase());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    #[cfg(feature = "ipadic")]
    use crate::{builder, token_filter::uppercase::UppercaseTokenFilter, DictionaryKind, Token};

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_uppercase_token_filter_apply() {
        let filter = UppercaseTokenFilter::default();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> =
            vec![
                Token::new("Rust", 0, 4, 0, WordId::default(), &dictionary, None)
                    .set_details(Some(vec!["UNK".to_string()]))
                    .clone(),
            ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].get_text(), "RUST");
    }
}
