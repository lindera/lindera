use std::io;

use clap::ArgMatches;

use crate::core::tokenizer::Tokenizer;

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

        // output result
        for token in tokenizer.tokenize(&text) {
            println!(
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
        }
        println!("EOS")
    }

    Ok(())
}
