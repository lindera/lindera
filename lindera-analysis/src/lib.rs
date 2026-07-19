//! Text analysis chain for Lindera.
//!
//! This crate layers Lucene-style text analysis on top of the pure
//! morphological segmenter provided by the [`lindera`] crate:
//!
//! - [`character_filter`]: transforms the input text before segmentation
//!   (with offset correction back to the original text)
//! - [`token_filter`]: transforms the tokens produced by the segmenter
//! - [`tokenizer`]: composes character filters, a
//!   [`Segmenter`](lindera::segmenter::Segmenter), and token filters into a
//!   single pipeline, configurable programmatically or via a YAML file

pub mod character_filter;
pub mod token_filter;
pub mod tokenizer;

use serde_json::Value;

use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;

/// Parses a CLI-style filter flag of the form `kind:{"arg": ...}` into the
/// filter kind and its JSON arguments.
///
/// # Arguments
///
/// * `cli_flag` - The flag string, e.g. `lowercase` or `length:{"max": 10}`.
///
/// # Returns
///
/// A tuple of the filter kind and the parsed JSON arguments.
fn parse_cli_flag(cli_flag: &str) -> LinderaResult<(&str, Value)> {
    let (kind, json) = cli_flag.split_once(':').unwrap_or((cli_flag, ""));

    let args: Value = serde_json::from_str(json)
        .map_err(|err| LinderaErrorKind::Content.with_error(anyhow::anyhow!(err)))?;

    Ok((kind, args))
}
