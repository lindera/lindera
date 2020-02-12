use crate::tokenizer::Token;

pub fn format_mecab(tokens: Vec<Token>) -> String {
    let mut lines = Vec::new();
    for token in tokens {
        let line = format!(
            "{}\t{},{},{},{},{},{},{},{},{}",
            token.text,
            token.detail.pos_level1,
            token.detail.pos_level2,
            token.detail.pos_level3,
            token.detail.pos_level4,
            token.detail.conjugation_type,
            token.detail.conjugate_form,
            token.detail.base_form,
            token.detail.reading,
            token.detail.pronunciation
        );
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

pub fn format(tokens: Vec<Token>, output_format: &str) -> Result<String, String> {
    return match output_format {
        "mecab" => Ok(format_mecab(tokens)),
        "wakati" => Ok(format_wakati(tokens)),
        "json" => Ok(format_json(tokens)),
        _ => Err(format!("unsupported output format: {}", output_format)),
    };
}
