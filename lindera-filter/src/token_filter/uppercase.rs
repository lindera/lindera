use lindera_core::LinderaResult;

use crate::{token::FilteredToken, token_filter::TokenFilter};

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

    fn apply(&self, tokens: &mut Vec<FilteredToken>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.text = token.text.to_uppercase();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        token::FilteredToken,
        token_filter::{uppercase::UppercaseTokenFilter, TokenFilter},
    };

    #[test]
    fn test_uppercase_token_filter_apply() {
        let filter = UppercaseTokenFilter::default();

        let mut tokens: Vec<FilteredToken> = vec![FilteredToken {
            text: "Rust".to_string(),
            byte_start: 0,
            byte_end: 4,
            position: 0,
            position_length: 1,
            details: vec!["UNK".to_string()],
        }];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].text, "RUST");
    }
}
