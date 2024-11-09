use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::{TokenFilter, TokenFilterConfig};
use crate::LinderaResult;

pub const UPPERCASE_TOKEN_FILTER_NAME: &str = "uppercase";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UppercaseTokenFilterConfig {}

impl UppercaseTokenFilterConfig {
    pub fn new() -> Self {
        Self {}
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<UppercaseTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

impl TokenFilterConfig for UppercaseTokenFilterConfig {
    fn from_value(value: &Value) -> LinderaResult<Self>
    where
        Self: Sized,
    {
        serde_json::from_value(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

impl Default for UppercaseTokenFilterConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalizes token text to uppercase.
///
#[derive(Clone, Debug)]
pub struct UppercaseTokenFilter {
    #[allow(dead_code)]
    config: UppercaseTokenFilterConfig,
}

impl UppercaseTokenFilter {
    pub fn new(config: UppercaseTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(UppercaseTokenFilterConfig::from_slice(data)?))
    }
}

impl TokenFilter for UppercaseTokenFilter {
    fn name(&self) -> &'static str {
        UPPERCASE_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            token.text = Cow::Owned(token.text.to_uppercase());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "ipadic")]
    fn test_uppercase_token_filter_apply() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::uppercase::UppercaseTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
        {}
        "#;

        let filter = UppercaseTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![Token {
            text: Cow::Borrowed("Rust"),
            byte_start: 0,
            byte_end: 4,
            position: 0,
            position_length: 1,
            word_id: WordId {
                id: 4294967295,
                is_system: true,
            },
            dictionary: &dictionary,
            user_dictionary: None,
            details: Some(vec![Cow::Borrowed("UNK")]),
        }];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].text, "RUST");
    }
}
