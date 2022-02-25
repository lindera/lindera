use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use clap::{AppSettings, Parser};

use lindera::formatter::format;
use lindera::formatter::Format;
use lindera::tokenizer::{Tokenizer, TokenizerConfig, UserDictionaryType};
use lindera_core::error::LinderaErrorKind;
use lindera_core::viterbi::{Mode, Penalty};
use lindera_core::LinderaResult;

/// Lindera CLI
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None, setting = AppSettings::DeriveDisplayOrder)]
struct Args {
    /// The dictionary direcory. If not specified, use the default dictionary.
    #[clap(short = 'd', long = "dict", value_name = "DICT")]
    dict: Option<PathBuf>,

    /// The user dictionary file path.
    #[clap(short = 'D', long = "user-dict", value_name = "USER_DICT")]
    user_dict: Option<PathBuf>,

    /// The user dictionary type. csv or bin
    #[clap(short = 't', long = "user-dict-type", value_name = "USER_DICT_TYPE")]
    user_dict_type: Option<String>,

    /// The tokenization mode. normal or search can be specified. If not specified, use the default mode.
    #[clap(short = 'm', long = "mode", value_name = "MODE")]
    mode: Option<String>,

    /// The output format. mecab, wakati or json can be specified. If not specified, use the default output format.
    #[clap(short = 'O', long = "output-format", value_name = "OUTPUT_FORMAT")]
    output_format: Option<String>,

    /// The input file path that contains the text for morphological analysis.
    #[clap(value_name = "INPUT_FILE")]
    input_file: Option<String>,
}

fn main() -> LinderaResult<()> {
    let args = Args::parse();

    // let mut config = TokenizerConfig::default();
    let mut config = TokenizerConfig {
        dict_path: args.dict,
        user_dict_path: args.user_dict,
        user_dict_type: UserDictionaryType::CSV,
        mode: Mode::Normal,
    };

    // user dictionary type
    match args.user_dict_type {
        Some(ref user_dict_type) => {
            if user_dict_type == "csv" {
                config.user_dict_type = UserDictionaryType::CSV;
            } else if user_dict_type == "bin" {
                config.user_dict_type = UserDictionaryType::Binary;
            } else {
                return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                    "Invalid user dictionary type: {}",
                    user_dict_type
                )));
            }
        }
        None => {
            config.user_dict_type = UserDictionaryType::CSV;
        }
    }

    // mode
    match args.mode {
        Some(mode) => match mode.as_str() {
            "normal" => config.mode = Mode::Normal,
            "search" => config.mode = Mode::Decompose(Penalty::default()),
            "decompose" => config.mode = Mode::Decompose(Penalty::default()),
            _ => {
                return Err(LinderaErrorKind::Args
                    .with_error(anyhow::anyhow!("unsupported mode: {}", mode)))
            }
        },
        None => config.mode = Mode::Normal,
    }

    // create tokenizer
    let tokenizer = Tokenizer::with_config(config)?;

    // output format
    let output_format = match args.output_format {
        Some(format) => match format.as_str() {
            "mecab" => Format::Mecab,
            "wakati" => Format::Wakati,
            "json" => Format::Json,
            _ => {
                return Err(LinderaErrorKind::Args
                    .with_error(anyhow::anyhow!("unsupported format: {}", format)))
            }
        },
        None => Format::Mecab,
    };

    let mut reader: Box<dyn BufRead> = if let Some(input_file) = args.input_file {
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
        match format(tokens, output_format) {
            Ok(output) => println!("{}", output),
            Err(msg) => println!("{}", msg),
        };
    }

    Ok(())
}
