use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, Read},
    path::PathBuf,
    str::FromStr,
};

use clap::{Parser, Subcommand};

use lindera::{
    analyzer::Analyzer,
    builder::{build_dictionary, build_user_dictionary},
    error::{LinderaError, LinderaErrorKind},
    mode::Mode,
    tokenizer::{
        DictionaryConfig, Tokenizer, TokenizerConfig, UserDictionaryConfig, CONTAINED_DICTIONARIES,
    },
    DictionaryKind, LinderaResult,
};

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

        match output_format {
            Format::Mecab => {
                let tokens = tokenizer.tokenize_with_details(&text)?;
                for token in tokens {
                    println!(
                        "{}\t{}",
                        token.text,
                        token
                            .details
                            .unwrap_or_default()
                            .iter()
                            .map(|d| d.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                }
                println!("EOS");
            }
            Format::Json => {
                let tokens = tokenizer.tokenize_with_details(&text)?;
                let mut tokens_json = Vec::new();
                for token in tokens {
                    let word_details = token.details.unwrap_or_default();
                    let token_info = serde_json::json!({
                        "text": token.text,
                        "details": word_details,
                        "byte_start": token.byte_start,
                        "byte_end": token.byte_end,
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
                let tokens = tokenizer.tokenize(&text)?;
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

        let tokens = analyzer.analyze(&text)?;
        match output_format {
            Format::Mecab => {
                for token in tokens {
                    println!(
                        "{}\t{}",
                        token.text,
                        token
                            .details
                            .unwrap_or_default()
                            .iter()
                            .map(|d| d.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                }
                println!("EOS");
            }
            Format::Json => {
                let mut tokens_json = Vec::new();
                for token in tokens {
                    let word_details = token.details.unwrap_or_default();
                    let token_info = serde_json::json!({
                        "text": token.text,
                        "details": word_details,
                        "byte_start": token.byte_start,
                        "byte_end": token.byte_end,
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
