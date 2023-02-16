use lindera_core::LinderaResult;

use crate::token::FilteredToken;

use super::TokenFilter;

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

    fn apply(&self, tokens: &mut Vec<FilteredToken>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.text = token.text.to_lowercase();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use crate::{
        token::FilteredToken, token_filter::lowercase::LowercaseTokenFilter,
        token_filter::TokenFilter,
    };

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_lowercase_token_filter_apply_ipadic() {
        let filter = LowercaseTokenFilter::default();

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
        assert_eq!(&tokens[0].text, "rust");
    }
}
