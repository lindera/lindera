use std::borrow::Cow;

use lindera_core::token_filter::TokenFilter;

use crate::{LinderaResult, Token};

pub const UPPERCASE_TOKEN_FILTER_NAME: &str = "uppercase";

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
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.text = Cow::Owned(token.text.to_uppercase());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{token_filter::uppercase::UppercaseTokenFilter, Token};

    #[test]
    fn test_uppercase_token_filter_apply() {
        let filter = UppercaseTokenFilter::default();

        let mut tokens: Vec<Token> = vec![Token {
            text: Cow::Borrowed("Rust"),
            details: None,
        }];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "RUST");
    }
}
