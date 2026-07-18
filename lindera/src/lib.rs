#[cfg(feature = "analysis")]
#[cfg_attr(docsrs, doc(cfg(feature = "analysis")))]
pub mod character_filter;
pub mod dictionary;
pub mod error;
pub mod mode;
pub mod segmenter;
pub mod token;
#[cfg(feature = "analysis")]
#[cfg_attr(docsrs, doc(cfg(feature = "analysis")))]
pub mod token_filter;
#[cfg(feature = "analysis")]
#[cfg_attr(docsrs, doc(cfg(feature = "analysis")))]
pub mod tokenizer;

#[cfg(feature = "analysis")]
use serde_json::Value;

#[cfg(feature = "analysis")]
use crate::error::LinderaErrorKind;

pub type LinderaResult<T> = lindera_dictionary::LinderaResult<T>;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERSION
}

#[cfg(feature = "analysis")]
fn parse_cli_flag(cli_flag: &str) -> LinderaResult<(&str, Value)> {
    let (kind, json) = cli_flag.split_once(':').unwrap_or((cli_flag, ""));

    let args: Value = serde_json::from_str(json)
        .map_err(|err| LinderaErrorKind::Content.with_error(anyhow::anyhow!(err)))?;

    Ok((kind, args))
}
