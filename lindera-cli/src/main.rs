use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use clap::{AppSettings, Parser};

use lindera::error::LinderaErrorKind;
use lindera::formatter::format;
use lindera::formatter::Format;
use lindera::mode::Mode;
use lindera::tokenizer::DEFAULT_DICTIONARY_TYPE;
use lindera::tokenizer::{DictionaryType, Tokenizer, TokenizerConfig, UserDictionaryType};
use lindera::LinderaResult;

/// Lindera CLI
#[derive(Parser, Debug)]
#[clap(version, about, setting = AppSettings::DeriveDisplayOrder)]
struct Args {
    /// The dictionary type. local and ipadic are available.
    #[clap(
        short = 't',
        long = "dict-type",
        value_name = "DICT_TYPE",
        default_value = DEFAULT_DICTIONARY_TYPE
    )]
    dict_type: String,

    /// The dictionary direcory. Specify the directory path of the dictionary when "local" is specified in the dictionary type.
    #[clap(short = 'd', long = "dict", value_name = "DICT")]
    dict: Option<PathBuf>,

    /// The user dictionary file path.
    #[clap(short = 'D', long = "user-dict", value_name = "USER_DICT")]
    user_dict: Option<PathBuf>,

    /// The user dictionary type. csv and bin are available.
    #[clap(short = 'T', long = "user-dict-type", value_name = "USER_DICT_TYPE")]
    user_dict_type: Option<String>,

    /// The tokenization mode. normal, search and decompose are available.
    #[clap(
        short = 'm',
        long = "mode",
        value_name = "MODE",
        default_value = "normal"
    )]
    mode: String,

    /// The output format. mecab, wakati or json can be specified. If not specified, use the default output format.
    #[clap(
        short = 'O',
        long = "output-format",
        value_name = "OUTPUT_FORMAT",
        default_value = "mecab"
    )]
    output_format: String,

    /// The input file path that contains the text for morphological analysis.
    #[clap(value_name = "INPUT_FILE")]
    input_file: Option<String>,
}

fn main() -> LinderaResult<()> {
    let args = Args::parse();

    // let mut config = TokenizerConfig::default();
    let mut config = TokenizerConfig {
        dict_type: DictionaryType::from_str(&args.dict_type)?,
        user_dict_path: args.user_dict,
        ..Default::default()
    };

    // user dictionary type
    match args.user_dict_type {
        Some(ref user_dict_type) => {
            config.user_dict_type = UserDictionaryType::from_str(user_dict_type)?;
        }
        None => {
            config.user_dict_type = UserDictionaryType::Csv;
        }
    }

    // mode
    config.mode = Mode::from_str(args.mode.as_str())?;

    // create tokenizer
    let tokenizer = Tokenizer::with_config(config)?;

    // output format
    let output_format = Format::from_str(args.output_format.as_str())?;

    // input file
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
