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
