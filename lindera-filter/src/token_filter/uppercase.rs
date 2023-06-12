use lindera_core::LinderaResult;
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

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
            token.text = token.text.to_uppercase().into();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_core::word_entry::WordId;
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
    #[cfg(any(all(feature = "ipadic",),))]
    use lindera_tokenizer::token::Token;

    #[cfg(any(all(feature = "ipadic",),))]
    use crate::token_filter::{uppercase::UppercaseTokenFilter, TokenFilter};

    #[test]
    #[cfg(any(all(feature = "ipadic",),))]
    fn test_uppercase_token_filter_apply() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let filter = UppercaseTokenFilter::default();

        let mut tokens: Vec<Token> = vec![Token::new(
            "Rust",
            0,
            9,
            0,
            WordId(4294967295, true),
            &dictionary,
            None,
        )];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].text, "RUST");
    }
}
