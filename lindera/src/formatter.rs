use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::tokenizer::Token;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    MeCab,
    Wakati,
    Json,
}

pub fn format_mecab(tokens: Vec<Token>) -> LinderaResult<String> {
    let mut lines = Vec::new();
    for token in tokens {
        let line = format!("{}\t{}", token.text, token.detail.join(","));
        lines.push(line);
    }
    lines.push(String::from("EOS"));

    Ok(lines.join("\n"))
}

pub fn format_wakati(tokens: Vec<Token>) -> LinderaResult<String> {
    let mut lines = Vec::new();
    for token in tokens {
        let line = token.text.to_string();
        lines.push(line);
    }

    Ok(lines.join(" "))
}

pub fn format_json(tokens: Vec<Token>) -> LinderaResult<String> {
    serde_json::to_string_pretty(&tokens)
        .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))
}

pub fn format(tokens: Vec<Token>, output_format: Format) -> LinderaResult<String> {
    return match output_format {
        Format::MeCab => format_mecab(tokens),
        Format::Wakati => format_wakati(tokens),
        Format::Json => format_json(tokens),
    };
}
