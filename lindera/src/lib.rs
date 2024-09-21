use serde_json::Value;

use crate::core::error::LinderaErrorKind;
use crate::core::LinderaResult;

pub mod character_filter;
pub mod core;
pub mod dictionary;
pub mod segmenter;
pub mod token;
pub mod token_filter;
pub mod tokenizer;

fn parse_cli_flag(cli_flag: &str) -> LinderaResult<(&str, Value)> {
    let (kind, json) = cli_flag.split_once(':').unwrap_or((cli_flag, ""));

    let args: Value = serde_json::from_str(json)
        .map_err(|err| LinderaErrorKind::Content.with_error(anyhow::anyhow!(err)))?;

    Ok((kind, args))
}
