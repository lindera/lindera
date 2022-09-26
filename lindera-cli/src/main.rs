use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use clap::{AppSettings, Parser, Subcommand};

use lindera::builder::{build_dictionary, build_user_dictionary};
use lindera::error::LinderaError;
use lindera::error::LinderaErrorKind;
use lindera::mode::Mode;
use lindera::tokenizer::DictionaryConfig;
use lindera::tokenizer::UserDictionaryConfig;
use lindera::tokenizer::DEFAULT_DICTIONARY_KIND;
use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use lindera::{DictionaryKind, LinderaResult};

#[derive(Debug, Parser)]
#[clap(name = "linera", author, about, version, setting = AppSettings::DeriveDisplayOrder)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Build(BuildArgs),
    Tokenize(TokenizeArgs),
}

#[derive(Debug, clap::Args)]
#[clap(author, about = "Tokenize text using a morphological analysis dictionary", version, setting = AppSettings::DeriveDisplayOrder)]
struct TokenizeArgs {
    #[clap(short = 't', long = "dic-type", default_value = DEFAULT_DICTIONARY_KIND, help = "Dictionary type")]
    dic_type: DictionaryKind,
    #[clap(short = 'd', long = "dic-dir", help = "Dictionary directory path")]
    dic_dir: Option<PathBuf>,
    #[clap(
        short = 'u',
        long = "user-dic-file",
        help = "User dictionary file path"
    )]
    user_dic_file: Option<PathBuf>,
    #[clap(
        short = 'm',
        long = "mode",
        default_value = "normal",
        help = "Tokenization mode. normal"
    )]
    mode: Mode,
    #[clap(
        short = 'o',
        long = "output-format",
        default_value = "mecab",
        help = "Output format"
    )]
    output_format: String,
    #[clap(help = "Input text file path")]
    input_file: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
#[clap(author, about = "Build a morphological analysis dictionary", version, setting = AppSettings::DeriveDisplayOrder)]
struct BuildArgs {
    #[clap(short = 'u', long = "build-user-dic", help = "Build user dictionary")]
    build_user_dic: bool,
    #[clap(short = 't', long = "dic-type", default_value = DEFAULT_DICTIONARY_KIND, help = "Dictionary type")]
    dic_type: DictionaryKind,
    #[clap(help = "Dictionary source path")]
    src_path: PathBuf,
    #[clap(help = "Dictionary destination path")]
    dest_path: PathBuf,
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

    match args.command {
        Commands::Tokenize(args) => tokenize(args),
        Commands::Build(args) => build(args),
    }
}

fn tokenize(args: TokenizeArgs) -> LinderaResult<()> {
    let dictionary_conf = DictionaryConfig {
        kind: args.dic_type.clone(),
        path: args.dic_dir,
    };

    let user_dictionary_conf = match args.user_dic_file {
        Some(path) => Some(UserDictionaryConfig {
            kind: args.dic_type,
            path,
        }),
        None => None,
    };

    let config = TokenizerConfig {
        dictionary: dictionary_conf,
        user_dictionary: user_dictionary_conf,
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

        // tokenize
        let tokens = tokenizer.tokenize(&text)?;

        match output_format {
            Format::Mecab => {
                // output result
                for token in tokens {
                    println!(
                        "{}\t{}",
                        token.text,
                        tokenizer.word_detail(token.word_id)?.join(",")
                    );
                }
                println!("EOS");
            }
            Format::Json => {
                // output result
                let mut tokens_json = Vec::new();
                for token in tokens {
                    let word_detail = tokenizer.word_detail(token.word_id)?;
                    let token_info = serde_json::json!({
                        "text": token.text,
                        "detail": word_detail,
                    });
                    tokens_json.push(token_info);
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&tokens_json).map_err(|err| {
                        LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err))
                    })?
                );
            }
            Format::Wakati => {
                // output result
                let mut it = tokens.iter().peekable();
                while let Some(token) = it.next() {
                    if it.peek().is_some() {
                        print!("{} ", token.text);
                    } else {
                        println!("{}", token.text);
                    }
                }
            }
        }
    }

    Ok(())
}

fn build(args: BuildArgs) -> LinderaResult<()> {
    if args.build_user_dic {
        build_user_dictionary(args.dic_type, &args.src_path, &args.dest_path)
    } else {
        build_dictionary(args.dic_type, &args.src_path, &args.dest_path)
    }
}
