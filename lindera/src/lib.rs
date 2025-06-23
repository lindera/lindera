pub mod character_filter;
pub mod dictionary;
pub mod error;
pub mod mode;
pub mod segmenter;
pub mod token;
pub mod token_filter;
pub mod tokenizer;

use serde_json::Value;

use crate::error::LinderaErrorKind;

pub type LinderaResult<T> = lindera_dictionary::LinderaResult<T>;

const VERERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERERSION
}

fn parse_cli_flag(cli_flag: &str) -> LinderaResult<(&str, Value)> {
    let (kind, json) = cli_flag.split_once(':').unwrap_or((cli_flag, ""));

    let args: Value = serde_json::from_str(json)
        .map_err(|err| LinderaErrorKind::Content.with_error(anyhow::anyhow!(err)))?;

    Ok((kind, args))
}

#[cfg(test)]
mod tests {
    /// Common test macros to reduce feature-specific repetition

    #[macro_export]
    macro_rules! feature_test {
        ($feature:literal, $test_name:ident, $test_body:block) => {
            #[test]
            #[cfg(feature = $feature)]
            fn $test_name() {
                $test_body
            }
        };
    }

    #[macro_export]
    macro_rules! dictionary_test {
        ($feature:literal, $dict_kind:expr, $test_name:ident, $test_body:expr) => {
            #[test]
            #[cfg(feature = $feature)]
            fn $test_name() {
                use crate::dictionary::{load_dictionary_from_kind, DictionaryKind};

                let dictionary = load_dictionary_from_kind($dict_kind).unwrap();
                $test_body(dictionary);
            }
        };
    }

    #[macro_export]
    macro_rules! token_filter_test {
        ($feature:literal, $dict_kind:expr, $test_name:ident, $filter_setup:expr, $test_body:expr) => {
            #[test]
            #[cfg(feature = $feature)]
            fn $test_name() {
                use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
                use crate::token::Token;
                use crate::token_filter::TokenFilter;
                use std::borrow::Cow;

                let dictionary = load_dictionary_from_kind($dict_kind).unwrap();
                let filter = $filter_setup;
                $test_body(filter, dictionary);
            }
        };
    }
}
