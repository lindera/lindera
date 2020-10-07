use crate::tokenizer::Token;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    MeCab,
    Wakati,
    JSON,
}

pub fn format_mecab(tokens: Vec<Token>) -> String {
    let mut lines = Vec::new();
    for token in tokens {
        let line = format!("{}\t{}", token.text, token.detail.join(","));
        lines.push(line);
    }
    lines.push(String::from("EOS"));

    lines.join("\n")
}

pub fn format_wakati(tokens: Vec<Token>) -> String {
    let mut lines = Vec::new();
    for token in tokens {
        let line = token.text.to_string();
        lines.push(line);
    }

    lines.join(" ")
}

pub fn format_json(tokens: Vec<Token>) -> String {
    serde_json::to_string_pretty(&tokens).unwrap()
}

pub fn format(tokens: Vec<Token>, output_format: Format) -> Result<String, String> {
    return match output_format {
        Format::MeCab => Ok(format_mecab(tokens)),
        Format::Wakati => Ok(format_wakati(tokens)),
        Format::JSON => Ok(format_json(tokens)),
    };
}
