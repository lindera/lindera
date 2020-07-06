#[macro_use]
extern crate clap;

use std::io::{BufRead, BufReader};
use std::{fs, io};

use clap::{App, AppSettings, Arg};
use lindera::formatter::format;
use lindera::tokenizer::Tokenizer;
use stringreader::StringReader;

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
            Arg::with_name("DICTIONARY_DIRECTORY")
                .help("The dictionary direcory. If not specified, use the default dictionary.")
                .short("d")
                .long("dictionary-directory")
                .value_name("DICTIONARY_DIRECTORY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("USER_DICTIONARY")
            .help("(Optional) The user dictionary file path.")
            .short("u")
            .long("userdic")
            .value_name("USER_DICTIONARY")
            .takes_value(true),
        )
        .arg(
            Arg::with_name("MODE")
                .help("The tokenization mode. `normal` or` search` can be specified. If not specified, use the default mode.")
                .short("m")
                .long("mode")
                .value_name("MODE")
                .default_value("normal")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("The output format. `mecab`, `wakati` or `json` can be specified. If not specified, use the default output format.")
                .short("o")
                .long("output")
                .value_name("OUTPUT")
                .default_value("mecab")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("INPUT_FILE")
                .help("The input file path that contains the text for morphological analysis.")
                .value_name("INPUT_FILE")
                .takes_value(true),
        );

    let matches = app.get_matches();

    // dictionary directory
    let mut dict_dir = "";
    if let Some(_dict_dir) = matches.value_of("DICTIONARY_DIRECTORY") {
        dict_dir = _dict_dir;
    }

    // user dictionary
    let mut user_dict = "";
    if let Some(_user_dict) = matches.value_of("USER_DICTIONARY") {
        user_dict = _user_dict;
    }

    // mode
    let mode_name = matches.value_of("MODE").unwrap();

    // create tokenizer
    let mut tokenizer = if user_dict.len() > 0 {
        Tokenizer::new_with_userdic(mode_name, dict_dir, user_dict)
    } else {
        Tokenizer::new(mode_name, dict_dir)
    };

    // output format
    let output_format = matches.value_of("OUTPUT").unwrap();

    if matches.is_present("INPUT_FILE") {
        let mut input_text = String::new();
        if let Some(f) = matches.value_of("INPUT_FILE") {
            match fs::read_to_string(f) {
                Ok(t) => {
                    input_text = t;
                }
                Err(e) => {
                    // return error message
                    println!("{}", e.to_string());
                    return;
                }
            }
        }

        let str_reader = StringReader::new(&input_text);
        let mut buf_reader = BufReader::new(str_reader);

        loop {
            // read the text to be tokenized from stdin
            let mut text = String::new();
            match buf_reader.read_line(&mut text) {
                Ok(_size) => {
                    if _size <= 0 {
                        // EOS
                        break;
                    }
                    text = text.trim().to_string();
                }
                Err(e) => {
                    // return error message
                    println!("{}", e.to_string());
                    return;
                }
            }

            // tokenize
            let tokens = tokenizer.tokenize(&text);

            // output result
            match format(tokens, output_format) {
                Ok(output) => println!("{}", output),
                Err(msg) => println!("{}", msg),
            };
        }
    } else {
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
                    println!("{}", e.to_string());
                    return;
                }
            }

            // tokenize
            let tokens = tokenizer.tokenize(&text);

            // output result
            match format(tokens, output_format) {
                Ok(output) => println!("{}", output),
                Err(msg) => println!("{}", msg),
            };
        }
    }
}
