use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::{Parser, Subcommand};

use lindera::LinderaResult;
use lindera::character_filter::CharacterFilterLoader;
use lindera::dictionary::{DictionaryKind, resolve_builder, resolve_metadata};
use lindera::error::{LinderaError, LinderaErrorKind};
use lindera::mode::Mode;
use lindera::token::Token;
use lindera::token_filter::TokenFilterLoader;
use lindera::tokenizer::TokenizerBuilder;
use lindera_cli::get_version;

#[derive(Debug, Parser)]
#[clap(
    name = env!("CARGO_BIN_NAME"),
    author,
    about = "A morphological analysis command line interface",
    version = get_version(),
)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    List(ListArgs),
    Tokenize(TokenizeArgs),
    Build(BuildArgs),
}

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "List a contained morphological analysis dictionaries",
    version = get_version(),
)]
struct ListArgs {}

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Tokenize text using a morphological analysis dictionary",
    version = get_version(),
)]
struct TokenizeArgs {
    #[clap(short = 'k', long = "dictionary-kind", help = "Kind of dictionary")]
    dic_type: Option<DictionaryKind>,
    #[clap(
        short = 'd',
        long = "dictionary-path",
        help = "Dictionary directory path"
    )]
    dic_dir: Option<PathBuf>,
    #[clap(
        short = 'u',
        long = "user-dictionary-path",
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
    #[clap(
        short = 'c',
        long = "character-filter",
        help = "Specify character filter. e.g. unicode_normalize:{\"kind\":\"NFKC\"}"
    )]
    character_filters: Option<Vec<String>>,
    #[clap(
        short = 't',
        long = "token-filter",
        help = "Specify token filter. e.g. stop_word:{\"words\":[\"a\", \"the\"]}"
    )]
    token_filters: Option<Vec<String>>,
    #[clap(help = "Input text file path")]
    input_file: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
#[clap(author,
    about = "Build a morphological analysis dictionary",
    version = get_version(),
)]
struct BuildArgs {
    #[clap(
        short = 'u',
        long = "build-user-dictionary",
        help = "Build user dictionary flag"
    )]
    build_user_dic: bool,
    #[clap(short = 'k', long = "dictionary-kind", help = "Kind of dictionary")]
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
        Commands::Build(args) => build(args),
    }
}

fn list(_args: ListArgs) -> LinderaResult<()> {
    for dic in DictionaryKind::contained_variants() {
        println!("{}", dic.as_str());
    }
    Ok(())
}

fn mecab_output(mut tokens: Vec<Token>) -> LinderaResult<()> {
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("{}\t{}", token.text.as_ref(), details);
    }
    println!("EOS");

    Ok(())
}

fn json_output(mut tokens: Vec<Token>) -> LinderaResult<()> {
    let mut json_tokens = Vec::new();
    for token in tokens.iter_mut() {
        let json_token = serde_json::json!({
            "text": token.text,
            "details": token.details(),
            "byte_start": token.byte_start,
            "byte_end": token.byte_end,
            "word_id": token.word_id,
        });
        json_tokens.push(json_token);
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json_tokens)
            .map_err(|err| { LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)) })?
    );

    Ok(())
}

fn wakati_output(tokens: Vec<Token>) -> LinderaResult<()> {
    let mut it = tokens.iter().peekable();
    while let Some(token) = it.next() {
        if it.peek().is_some() {
            print!("{} ", token.text.as_ref());
        } else {
            println!("{}", token.text.as_ref());
        }
    }

    Ok(())
}

fn tokenize(args: TokenizeArgs) -> LinderaResult<()> {
    let mut builder = TokenizerBuilder::new()?;

    // Set kind of dictionary
    if let Some(ref dic_type) = args.dic_type {
        builder.set_segmenter_dictionary_kind(dic_type);
    }
    // Set dictionary directory path
    if let Some(dic_dir) = args.dic_dir {
        builder.set_segmenter_dictionary_path(dic_dir.as_path());
    }

    // Set user dictionary file path
    if let Some(user_dic_file) = args.user_dic_file {
        builder.set_segmenter_user_dictionary_path(user_dic_file.as_path());

        // If user dictionary file path is specified, set kind of user dictionary or not
        if let Some(ref dic_type) = args.dic_type {
            builder.set_segmenter_user_dictionary_kind(dic_type);
        }
    }

    // Mode
    builder.set_segmenter_mode(&args.mode);

    // Tokenizer
    let mut tokenizer = builder
        .build()
        .map_err(|err| LinderaErrorKind::Args.with_error(err))?;

    // output format
    let output_format = Format::from_str(args.output_format.as_str())?;

    // Character flters
    for filter in args.character_filters.iter().flatten() {
        let character_filter = CharacterFilterLoader::load_from_cli_flag(filter)?;
        tokenizer.append_character_filter(character_filter);
    }

    // Token filters
    for filter in args.token_filters.iter().flatten() {
        let token_filter = TokenFilterLoader::load_from_cli_flag(filter)?;
        tokenizer.append_token_filter(token_filter);
    }

    // input file
    let mut reader: Box<dyn BufRead> = if let Some(input_file) = args.input_file {
        Box::new(BufReader::new(File::open(input_file).map_err(|err| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(err))
        })?))
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

        let tokens = tokenizer.tokenize(text.trim())?;

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
    let metadata = resolve_metadata(args.dic_type.clone())?;
    let builder = resolve_builder(args.dic_type)?;

    if args.build_user_dic {
        let output_file = if let Some(filename) = args.src_path.file_name() {
            let mut output_file = Path::new(&args.dest_path).join(filename);
            output_file.set_extension("bin");
            output_file
        } else {
            return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!("failed to get filename")));
        };
        builder.build_user_dictionary(&metadata, &args.src_path, &output_file)
    } else {
        builder.build_dictionary(&metadata, &args.src_path, &args.dest_path)
    }
}
