use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::str::FromStr;

use lindera::LinderaResult;
use lindera::error::{LinderaError, LinderaErrorKind};
use lindera::mode::Mode;
use lindera::token::Token;
use lindera_analysis::character_filter::CharacterFilterLoader;
use lindera_analysis::token_filter::TokenFilterLoader;
use lindera_analysis::tokenizer::TokenizerBuilder;
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
        help = "Dictionary directory path, URI, or downloaded dictionary name (e.g., embedded://ipadic, /path/to/dictionary, ipadic)"
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
        long = "max-grouping-len",
        help = "Maximum unknown-word grouping length in characters beyond the first (MeCab's max-grouping-size; MeCab defaults to 24). Longer runs fall back to single-character unknown words. Default: unbounded"
    )]
    max_grouping_len: Option<usize>,
    #[clap(
        long = "mmap",
        help = "Use memory-mapped file loading for the dictionary directory's word list. Ignored for embedded:// dictionaries and when the mmap feature is disabled. Rebuilding or truncating the dictionary directory while a process holds it mapped can cause a SIGBUS on the next lookup."
    )]
    use_mmap: bool,
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

/// Writes tokens to the given writer in the requested output format.
///
/// # Arguments
///
/// * `writer` - The destination for the formatted output (typically a buffered stdout lock).
/// * `format` - The output format to render the tokens in.
/// * `tokens` - The tokens produced for one input line.
/// * `details_buf` - A scratch buffer reused across tokens for joining detail fields.
///
/// # Returns
///
/// `Ok(())` on success, or an I/O / serialization error wrapped in `LinderaError`.
fn write_output<W: Write>(
    writer: &mut W,
    format: Format,
    tokens: Vec<Token>,
    details_buf: &mut String,
) -> LinderaResult<()> {
    match format {
        Format::Mecab => mecab_output(writer, tokens, details_buf),
        Format::Json => json_output(writer, tokens),
        Format::Wakati => wakati_output(writer, tokens),
    }
}

/// Writes tokens in the MeCab format: one `surface\tdetails` line per token,
/// terminated by an `EOS` line.
///
/// # Arguments
///
/// * `writer` - The destination for the formatted output.
/// * `tokens` - The tokens produced for one input line.
/// * `details_buf` - A scratch buffer reused across tokens for joining detail fields.
///
/// # Returns
///
/// `Ok(())` on success, or an I/O error wrapped in `LinderaError`.
fn mecab_output<W: Write>(
    writer: &mut W,
    mut tokens: Vec<Token>,
    details_buf: &mut String,
) -> LinderaResult<()> {
    for token in tokens.iter_mut() {
        details_buf.clear();
        // details_iter avoids the fresh Vec<&str> that details() collects
        // on every call (#942); the joined string reuses details_buf.
        for (i, detail) in token.details_iter().enumerate() {
            if i > 0 {
                details_buf.push(',');
            }
            details_buf.push_str(detail);
        }
        writeln!(writer, "{}\t{}", token.surface.as_ref(), details_buf).map_err(io_err)?;
    }
    writeln!(writer, "EOS").map_err(io_err)?;

    Ok(())
}

/// Writes tokens as a pretty-printed JSON array of token objects.
///
/// # Arguments
///
/// * `writer` - The destination for the formatted output.
/// * `tokens` - The tokens produced for one input line.
///
/// # Returns
///
/// `Ok(())` on success, or an I/O / serialization error wrapped in `LinderaError`.
fn json_output<W: Write>(writer: &mut W, mut tokens: Vec<Token>) -> LinderaResult<()> {
    let mut json_tokens = Vec::new();
    for token in tokens.iter_mut() {
        let token_value = token.as_value();
        json_tokens.push(token_value);
    }

    serde_json::to_writer_pretty(&mut *writer, &json_tokens)
        .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
    writeln!(writer).map_err(io_err)?;

    Ok(())
}

/// Writes tokens in the wakati format: surfaces separated by single spaces on
/// one line.
///
/// # Arguments
///
/// * `writer` - The destination for the formatted output.
/// * `tokens` - The tokens produced for one input line.
///
/// # Returns
///
/// `Ok(())` on success, or an I/O error wrapped in `LinderaError`.
fn wakati_output<W: Write>(writer: &mut W, tokens: Vec<Token>) -> LinderaResult<()> {
    let mut it = tokens.iter().peekable();
    while let Some(token) = it.next() {
        if it.peek().is_some() {
            write!(writer, "{} ", token.surface.as_ref()).map_err(io_err)?;
        } else {
            writeln!(writer, "{}", token.surface.as_ref()).map_err(io_err)?;
        }
    }

    Ok(())
}

pub fn tokenize(args: TokenizeArgs) -> LinderaResult<()> {
    let mut builder = TokenizerBuilder::new()?;

    // Set dictionary directory URI. A bare downloadable dictionary name
    // (e.g. `ipadic`) resolves to its downloaded directory; URIs and
    // existing filesystem paths are passed through unchanged.
    let dict_uri = crate::dictionary_registry::resolve_dictionary_arg(
        args.dict.as_str(),
        std::env::var_os(crate::dictionary_registry::DATA_DIR_ENV).map(PathBuf::from),
        get_version(),
    )?;
    builder.set_segmenter_dictionary(dict_uri.as_str());

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

    // Unknown-word grouping cap (default: unbounded)
    if let Some(max_grouping_len) = args.max_grouping_len {
        builder.set_segmenter_max_grouping_len(max_grouping_len);
    }

    // Memory-mapped dictionary loading (ignored for embedded:// dictionaries)
    if args.use_mmap {
        builder.set_segmenter_use_mmap(true);
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

    // Reusable analysis session: keeps the lattice, the backtrace scratch,
    // and the character-filtered text buffer alive across lines, so token
    // surfaces stay borrowed (no per-token String) even when character
    // filters are configured (#942).
    let mut worker = tokenizer.into_worker();

    // Buffer all output on a locked stdout: the default line-buffered stdout
    // would otherwise issue one write syscall per output line.
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    // Reused across every line/token to avoid per-line and per-token
    // reallocations.
    let mut text = String::new();
    let mut details_buf = String::new();

    loop {
        // read the text to be tokenized from stdin
        text.clear();
        let size = reader.read_line(&mut text).map_err(io_err)?;
        if size == 0 {
            // EOS
            break;
        }

        if nbest >= 2 {
            let results =
                worker.tokenize_nbest(text.trim(), nbest, nbest_unique, nbest_cost_threshold)?;
            for (rank, (tokens, cost)) in results.into_iter().enumerate() {
                writeln!(writer, "NBEST {} (cost={})", rank + 1, cost).map_err(io_err)?;
                write_output(&mut writer, output_format, tokens, &mut details_buf)?;
            }
        } else {
            let tokens = worker.tokenize(text.trim())?;
            write_output(&mut writer, output_format, tokens, &mut details_buf)?;
        }
    }

    // Surface write errors instead of letting the implicit flush on drop
    // swallow them.
    writer.flush().map_err(io_err)?;

    Ok(())
}
