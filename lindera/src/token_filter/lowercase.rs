use std::borrow::Cow;

use lindera_core::token_filter::TokenFilter;

use crate::{LinderaResult, Token};

pub const LOWERCASE_TOKEN_FILTER_NAME: &str = "lowercase";

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
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.text = Cow::Owned(token.text.to_lowercase());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{token_filter::lowercase::LowercaseTokenFilter, Token};

    #[test]
    fn test_lowercase_token_filter_apply() {
        let filter = LowercaseTokenFilter::default();

        let mut tokens: Vec<Token> = vec![Token {
            text: Cow::Borrowed("Rust"),
            details: None,
        }];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "rust");
    }
}
