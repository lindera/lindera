use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use clap::{AppSettings, Parser};

use lindera::error::LinderaError;
use lindera::error::LinderaErrorKind;
use lindera::mode::Mode;
use lindera::tokenizer::DictionaryConfig;
use lindera::tokenizer::DictionaryKind;
use lindera::tokenizer::DictionarySourceType;
use lindera::tokenizer::UserDictionaryConfig;
use lindera::tokenizer::DEFAULT_DICTIONARY_KIND;
use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use lindera::LinderaResult;

/// Lindera CLI
#[derive(Parser, Debug)]
#[clap(version, about, setting = AppSettings::DeriveDisplayOrder)]
struct Args {
    /// The dictionary type.
    #[clap(
        short = 'k',
        long = "dictionary-kind",
        value_name = "DICTIONARY_KIND",
        default_value = DEFAULT_DICTIONARY_KIND
    )]
    dictionary_kind: DictionaryKind,

    /// Directory path of the dictionary. If specified, loads the specified directory as the specified dictionary kind. If omitted, the self-contained dictionary specified by dictionary kind is loaded.
    #[clap(short = 'd', long = "dictionary-path", value_name = "DICTIONARY_PATH")]
    dictionary_path: Option<PathBuf>,

    /// The user dictionary file path. If specified, loads the specified file as a user dictionary.
    #[clap(
        short = 'u',
        long = "user-dictionary-path",
        value_name = "USER_DICTIONARY_PATH"
    )]
    user_dictionary_path: Option<PathBuf>,

    /// The user dictionary source type. Enabled when a user dictionary is specified. Default is "csv".
    #[clap(
        short = 't',
        long = "user-dictionary-source-type",
        value_name = "USER_DICTIONARY_SOURCE_TYPE",
        default_value = "csv"
    )]
    user_dictionary_source_type: DictionarySourceType,

    /// The tokenization mode. normal, search and decompose are available.
    #[clap(
        short = 'm',
        long = "mode",
        value_name = "MODE",
        default_value = "normal"
    )]
    mode: Mode,

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

#[derive(Debug, Clone, Copy)]
/// Formatter type
pub enum Format {
    Mecab,
    Wakati,
    Json,
}

impl FromStr for Format {
    type Err = LinderaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mecab" => Ok(Format::Mecab),
            "wakati" => Ok(Format::Wakati),
            "json" => Ok(Format::Json),
            _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!("Invalid format: {}", s))),
        }
    }
}

fn main() -> LinderaResult<()> {
    let args = Args::parse();

    let dictionary_meta = DictionaryConfig {
        kind: args.dictionary_kind.clone(),
        path: args.dictionary_path,
    };

    let user_dictionary_meta = match args.user_dictionary_path {
        Some(path) => Some(UserDictionaryConfig {
            kind: args.dictionary_kind,
            source_type: args.user_dictionary_source_type,
            path,
        }),
        None => None,
    };

    let config = TokenizerConfig {
        dictionary: dictionary_meta,
        user_dictionary: user_dictionary_meta,
        mode: args.mode,
    };

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

        match output_format {
            Format::Mecab => {
                // tokenize
                let tokens = tokenizer.tokenize(&text)?;

                // output result
                for token in tokens {
                    println!("{}\t{}", token.text, token.detail.join(","));
                }
                println!("EOS");
            }
            Format::Json => {
                // tokenize
                let tokens = tokenizer.tokenize(&text)?;

                // output result
                println!(
                    "{}",
                    serde_json::to_string_pretty(&tokens).map_err(|err| {
                        LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err))
                    })?
                );
            }
            Format::Wakati => {
                // tokenize
                let tokens = tokenizer.tokenize_str(&text)?;

                // output result
                let mut it = tokens.iter().peekable();
                while let Some(token) = it.next() {
                    if it.peek().is_some() {
                        print!("{} ", token);
                    } else {
                        println!("{}", token);
                    }
                }
            }
        }
    }

    Ok(())
}
