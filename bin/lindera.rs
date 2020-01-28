#[macro_use]
extern crate clap;

use std::io;

use clap::{App, AppSettings, Arg};

use lindera::core::tokenizer::Tokenizer;

fn main() {
    let app = App::new(crate_name!())
        .setting(AppSettings::DeriveDisplayOrder)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .help_message("Prints help information.")
        .version_message("Prints version information.")
        .version_short("v")
        .arg(
            Arg::with_name("MODE")
                .help("Tokenization mode. `normal` or` search` can be specified. If not specified, use the default mode.")
                .short("m")
                .long("mode")
                .value_name("MODE")
                .default_value("normal")
                .takes_value(true),
        );

    let matches = app.get_matches();

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
            panic!("unsupported mode: {}", mode);
        }
    }

    loop {
        let mut text = String::new();
        io::stdin()
            .read_line(&mut text)
            .expect("Failed to read line");
        text = text.trim().to_string();

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
}
