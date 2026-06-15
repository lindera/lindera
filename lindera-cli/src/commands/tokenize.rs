use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use lindera::LinderaResult;
use lindera::character_filter::CharacterFilterLoader;
use lindera::error::{LinderaError, LinderaErrorKind};
use lindera::mode::Mode;
use lindera::token::Token;
use lindera::token_filter::TokenFilterLoader;
use lindera::tokenizer::TokenizerBuilder;
use lindera_cli::get_version;

use super::io_err;

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Tokenize text using a morphological analysis dictionary",
    version = get_version(),
)]
pub struct TokenizeArgs {
    #[clap(
        short = 'd',
        long = "dict",
        required = true,
        help = "Dictionary directory path or URI (e.g., embedded://ipadic, /path/to/dictionary)"
    )]
    dict: String,
    #[clap(
        short = 'o',
        long = "output",
        default_value = "mecab",
        help = "Output format (mecab|wakati|json)"
    )]
    output: String,
    #[clap(
        short = 'u',
        long = "user-dict",
        help = "User dictionary path or URI (optional)"
    )]
    user_dict: Option<String>,
    #[clap(
        short = 'm',
        long = "mode",
        default_value = "normal",
        help = "Tokenization mode (normal|decompose)"
    )]
    mode: Mode,
    #[clap(
        short = 'c',
        long = "char-filter",
        help = "Character filter config (JSON)"
    )]
    character_filters: Option<Vec<String>>,
    #[clap(
        short = 't',
        long = "token-filter",
        help = "Token filter config (JSON)"
    )]
    token_filters: Option<Vec<String>>,
    #[clap(
        long = "keep-whitespace",
        help = "Keep whitespace tokens in output (default: whitespace is ignored for MeCab compatibility)"
    )]
    keep_whitespace: bool,
    #[clap(
        short = 'N',
        long = "nbest",
        default_value = "1",
        help = "Number of N-best results (default: 1)"
    )]
    nbest: usize,
    #[clap(
        long = "nbest-unique",
        help = "Deduplicate N-best results with the same word boundaries (keeps only the lowest-cost POS variant)"
    )]
    nbest_unique: bool,
    #[clap(
        long = "nbest-cost-threshold",
        help = "Maximum cost difference from best path for N-best results (e.g. 10000)"
    )]
    nbest_cost_threshold: Option<i64>,
    #[clap(help = "Input text file (default: stdin)")]
    input_file: Option<PathBuf>,
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
            _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!("Invalid format: {s}"))),
        }
    }
}

/// Writes tokens to stdout in the requested output format.
fn write_output(format: Format, tokens: Vec<Token>) -> LinderaResult<()> {
    match format {
        Format::Mecab => mecab_output(tokens),
        Format::Json => json_output(tokens),
        Format::Wakati => wakati_output(tokens),
    }
}

fn mecab_output(mut tokens: Vec<Token>) -> LinderaResult<()> {
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("{}\t{}", token.surface.as_ref(), details);
    }
    println!("EOS");

    Ok(())
}

fn json_output(mut tokens: Vec<Token>) -> LinderaResult<()> {
    let mut json_tokens = Vec::new();
    for token in tokens.iter_mut() {
        let token_value = token.as_value();
        json_tokens.push(token_value);
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
            print!("{} ", token.surface.as_ref());
        } else {
            println!("{}", token.surface.as_ref());
        }
    }

    Ok(())
}

pub fn tokenize(args: TokenizeArgs) -> LinderaResult<()> {
    let mut builder = TokenizerBuilder::new()?;

    // Set dictionary directory URI
    builder.set_segmenter_dictionary(args.dict.as_str());

    // Set user dictionary URI
    if let Some(user_dic_uri) = args.user_dict {
        builder.set_segmenter_user_dictionary(user_dic_uri.as_str());
    }

    // Mode
    builder.set_segmenter_mode(&args.mode);

    // Keep whitespace (default is to ignore whitespace for MeCab compatibility)
    if args.keep_whitespace {
        builder.set_segmenter_keep_whitespace(true);
    }

    // Tokenizer
    let mut tokenizer = builder
        .build()
        .map_err(|err| LinderaErrorKind::Args.with_error(err))?;

    // output format
    let output_format = Format::from_str(args.output.as_str())?;

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
        Box::new(BufReader::new(File::open(input_file).map_err(io_err)?))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };

    let nbest = args.nbest;
    let nbest_unique = args.nbest_unique;
    let nbest_cost_threshold = args.nbest_cost_threshold;

    loop {
        // read the text to be tokenized from stdin
        let mut text = String::new();
        let size = reader.read_line(&mut text).map_err(io_err)?;
        if size == 0 {
            // EOS
            break;
        }

        if nbest >= 2 {
            let results =
                tokenizer.tokenize_nbest(text.trim(), nbest, nbest_unique, nbest_cost_threshold)?;
            for (rank, (tokens, cost)) in results.into_iter().enumerate() {
                println!("NBEST {} (cost={})", rank + 1, cost);
                write_output(output_format, tokens)?;
            }
        } else {
            let tokens = tokenizer.tokenize(text.trim())?;
            write_output(output_format, tokens)?;
        }
    }

    Ok(())
}
