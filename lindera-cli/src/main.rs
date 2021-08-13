use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;

use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};

use lindera::formatter::{format, Format};
use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use lindera_core::error::LinderaErrorKind;
use lindera_core::viterbi::{Mode, Penalty};
use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    let app = App::new(crate_name!())
        .setting(AppSettings::DeriveDisplayOrder)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .help_message("Prints help information.")
        .version_message("Prints version information.")
        .version_short("v")
        .arg(
            Arg::with_name("DICTIONARY")
                .help("The dictionary direcory. If not specified, use the default dictionary.")
                .short("d")
                .long("dictionary")
                .value_name("DICTIONARY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("USER_DICTIONARY")
            .help("(Optional) The user dictionary file path.")
            .short("u")
            .long("user-dictionary")
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
            Arg::with_name("OUTPUT_FORMAT")
                .help("The output format. `mecab`, `wakati` or `json` can be specified. If not specified, use the default output format.")
                .short("O")
                .long("output-format")
                .value_name("OUTPUT_FORMAT")
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

    let mut config = TokenizerConfig::default();
    // dictionary directory
    if let Some(dict_dir) = matches.value_of("DICTIONARY") {
        config.dict_path = Some(Path::new(dict_dir));
    }

    // user dictionary
    if let Some(user_dict) = matches.value_of("USER_DICTIONARY") {
        config.user_dict_path = Some(Path::new(user_dict));
    }

    // mode
    let mode_name = matches.value_of("MODE").unwrap();
    match mode_name {
        "normal" => config.mode = Mode::Normal,
        "decompose" => config.mode = Mode::Decompose(Penalty::default()),
        _ => {
            // show error message
            return Err(LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("unsupported mode: {}", mode_name)));
        }
    }

    // create tokenizer
    let mut tokenizer = Tokenizer::with_config(config)?;

    // output format
    let output_format = matches.value_of("OUTPUT_FORMAT").unwrap();
    let f = match output_format {
        "mecab" => Format::Mecab,
        "wakati" => Format::Wakati,
        "json" => Format::Json,
        _ => {
            // show error message
            return Err(LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("unsupported format: {}", output_format)));
        }
    };

    let mut reader: Box<dyn BufRead> = if let Some(input_file) = matches.value_of("INPUT_FILE") {
        Box::new(BufReader::new(fs::File::open(input_file).map_err(
            |err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)),
        )?))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };

    loop {
        // read the text to be tokenized from stdin
        let mut text = String::new();
        let size = reader
            .read_line(&mut text)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
        if size == 0 {
            // EOS
            break;
        }
        text = text.trim().to_string();

        // tokenize
        let tokens = tokenizer.tokenize(&text)?;

        // output result
        match format(tokens, f) {
            Ok(output) => println!("{}", output),
            Err(msg) => println!("{}", msg),
        };
    }

    Ok(())
}
