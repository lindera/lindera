use std::{
    fs,
    io::{self, BufRead, BufReader},
    path::PathBuf,
    str::FromStr,
};

#[cfg(feature = "filter")]
use std::{fs::File, io::Read};

use clap::{Parser, Subcommand};

#[cfg(feature = "filter")]
use lindera_analyzer::analyzer::Analyzer;
use lindera_analyzer::token::Token;

use lindera_core::{
    error::{LinderaError, LinderaErrorKind},
    mode::Mode,
    LinderaResult,
};
use lindera_dictionary::{
    build_dictionary, build_user_dictionary, DictionaryConfig, DictionaryKind, UserDictionaryConfig,
};
use lindera_tokenizer::tokenizer::{Tokenizer, TokenizerConfig, CONTAINED_DICTIONARIES};

#[derive(Debug, Parser)]
#[clap(name = "linera", author, about, version)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    List(ListArgs),
    Tokenize(TokenizeArgs),
    #[cfg(feature = "filter")]
    Analyze(AnalyzeArgs),
    Build(BuildArgs),
}

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "List a contained morphological analysis dictionaries",
    version
)]
struct ListArgs {}

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Tokenize text using a morphological analysis dictionary",
    version
)]
struct TokenizeArgs {
    #[clap(short = 't', long = "dic-type", help = "Dictionary type")]
    dic_type: Option<DictionaryKind>,
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

#[cfg(feature = "filter")]
#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Analyze text with character filters, tokenizer and token filters ",
    version
)]
struct AnalyzeArgs {
    #[clap(short = 'c', long = "config", help = "Configuration file path")]
    config_path: PathBuf,
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
#[clap(author, about = "Build a morphological analysis dictionary", version)]
struct BuildArgs {
    #[clap(short = 'u', long = "build-user-dic", help = "Build user dictionary")]
    build_user_dic: bool,
    #[clap(short = 't', long = "dic-type", help = "Dictionary type")]
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
        Commands::List(args) => list(args),
        Commands::Tokenize(args) => tokenize(args),
        #[cfg(feature = "filter")]
        Commands::Analyze(args) => analyze(args),
        Commands::Build(args) => build(args),
    }
}

fn list(_args: ListArgs) -> LinderaResult<()> {
    for dic in CONTAINED_DICTIONARIES {
        println!("{}", dic);
    }
    Ok(())
}

fn mecab_output(mut tokens: Vec<Token>) -> LinderaResult<()> {
    for token in tokens.iter_mut() {
        println!("{}\t{}", token.text.clone(), token.details.join(","));
    }
    println!("EOS");

    Ok(())
}

fn json_output(mut tokens: Vec<Token>) -> LinderaResult<()> {
    let mut tokens_json = Vec::new();
    for token in tokens.iter_mut() {
        let token_info = serde_json::json!({
            "text": token.text.clone(),
            "details": token.details,
            "byte_start": token.byte_start,
            "byte_end": token.byte_end,
            "word_id": token.word_id,
        });
        tokens_json.push(token_info);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&tokens_json)
            .map_err(|err| { LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)) })?
    );

    Ok(())
}

fn wakati_output(tokens: Vec<Token>) -> LinderaResult<()> {
    let mut it = tokens.iter().peekable();
    while let Some(token) = it.next() {
        if it.peek().is_some() {
            print!("{} ", token.text);
        } else {
            println!("{}", token.text);
        }
    }

    Ok(())
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
    let tokenizer = Tokenizer::from_config(config)?;

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

        let mut tmp_tokens = tokenizer.tokenize(text.trim())?;
        let mut tokens = Vec::new();
        for token in tmp_tokens.iter_mut() {
            tokens.push(Token {
                text: token.text.to_string(),
                byte_start: token.byte_start,
                byte_end: token.byte_end,
                position: token.position,
                position_length: token.position_length,
                word_id: token.word_id,
                details: token
                    .get_details()
                    .ok_or_else(|| {
                        LinderaErrorKind::Content.with_error(anyhow::anyhow!("unknown error"))
                    })?
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            });
        }
        match output_format {
            Format::Mecab => {
                mecab_output(tokens)?;
            }
            Format::Json => {
                json_output(tokens)?;
            }
            Format::Wakati => {
                wakati_output(tokens)?;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "filter")]
fn analyze(args: AnalyzeArgs) -> LinderaResult<()> {
    let mut config_file = File::open(args.config_path)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    let mut config_bytes = Vec::new();
    config_file
        .read_to_end(&mut config_bytes)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    let analyzer = Analyzer::from_slice(&config_bytes)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

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
        let text = text.trim().to_string();
        let tokens = analyzer.analyze(&mut &text)?;
        match output_format {
            Format::Mecab => {
                mecab_output(tokens)?;
            }
            Format::Json => {
                json_output(tokens)?;
            }
            Format::Wakati => {
                wakati_output(tokens)?;
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
