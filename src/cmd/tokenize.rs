use std::io;

use clap::ArgMatches;
use lindera::tokenizer::tokenizer::{Token, Tokenizer};

fn format_mecab(tokens: Vec<Token>) -> String {
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

fn format_wakati(tokens: Vec<Token>) -> String {
    let mut lines = Vec::new();
    for token in tokens {
        let line = token.text.to_string();
        lines.push(line);
    }

    lines.join(" ")
}

fn format_json(tokens: Vec<Token>) -> String {
    serde_json::to_string_pretty(&tokens).unwrap()
}

pub fn run_tokenize_cli(matches: &ArgMatches) -> Result<(), String> {
    // create tokenizer
    let mode = matches.value_of("MODE").unwrap();
    let mut tokenizer;
    match mode {
        "normal" => {
            tokenizer = Tokenizer::normal();
        }
        "search" => {
            tokenizer = Tokenizer::for_search();
        }
        _ => {
            return Err(format!("unsupported mode: {}", mode));
        }
    }

    // output format
    let output_format = matches.value_of("OUTPUT").unwrap();

    loop {
        // read the text to be tokenized from stdin
        let mut text = String::new();
        match io::stdin().read_line(&mut text) {
            Ok(_size) => {
                if _size <= 0 {
                    // EOS
                    break;
                }
                text = text.trim().to_string();
            }
            Err(e) => {
                // return error message
                return Err(e.to_string());
            }
        }

        // tokenize
        let tokens = tokenizer.tokenize(&text);

        // output result
        match output_format {
            "mecab" => {
                println!("{}", format_mecab(tokens));
            }
            "wakati" => {
                println!("{}", format_wakati(tokens));
            }
            "json" => {
                println!("{}", format_json(tokens));
            }
            _ => {
                return Err(format!("unsupported output format: {}", mode));
            }
        }
    }

    Ok(())
}
