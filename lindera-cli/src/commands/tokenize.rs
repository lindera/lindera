use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::str::FromStr;

use lindera::LinderaResult;
use lindera::analysis::character_filter::CharacterFilterLoader;
use lindera::analysis::token_filter::TokenFilterLoader;
use lindera::analysis::tokenizer::TokenizerBuilder;
use lindera::error::{LinderaError, LinderaErrorKind};
use lindera::mode::Mode;
use lindera::space_penalty::SpacePenaltyConfig;
use lindera::token::Token;
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
        help = "Keep whitespace tokens in output. Whitespace then also stays in the lattice, so the other tokens can differ from the default output, in which whitespace is dropped and, unless the dictionary's metadata.json turns it off (SudachiDict does), skipped in the lattice as MeCab does"
    )]
    keep_whitespace: bool,
    #[clap(
        long = "disable-skip-whitespace",
        help = "Keep whitespace in the lattice as a node of its own (a whitespace dictionary entry or the SPACE unknown word) instead of skipping it, while still dropping it from the output (the behavior before whitespace skipping). By default the words on either side of whitespace connect directly, as in MeCab, for every dictionary whose metadata.json does not turn skipping off (SudachiDict does). No effect with --keep-whitespace"
    )]
    disable_skip_whitespace: bool,
    #[clap(
        long = "max-grouping-len",
        help = "Cap on unknown-word grouping, in characters beyond the first (MeCab's max-grouping-size; MeCab defaults to 24). Applied at each position: a same-category run longer than the cap is not grouped there; the single-character candidate (plus the length ladder and dictionary words) remains and the remaining tail is grouped again once it fits, so no unknown token exceeds cap+1 characters. 0 or omitted: unbounded"
    )]
    max_grouping_len: Option<usize>,
    #[clap(
        long = "disable-unknown-word-ladder",
        help = "Disable the MeCab/Vibrato-inspired unknown-word length ladder (char.def's LENGTH field), leaving out these candidates as before v6; other changes since then, such as unknown words that start inside a grouped run, still apply. Enabled by default"
    )]
    disable_unknown_word_ladder: bool,
    #[clap(
        long = "disable-space-penalty",
        conflicts_with = "space_penalty_rules",
        help = "Disable the left-space penalty (mecab-ko's left-space-penalty-factor) that a dictionary shipping rules in its metadata.json applies by default (ko-dic). A candidate that starts right after whitespace and whose first part-of-speech tag is listed gets the cost added; disabling it together with --disable-skip-whitespace turns off both changes since v6.0 in how Korean is read around spaces (this penalty and whitespace skipping), but not the other output changes since then, such as the split of sentence-final punctuation runs"
    )]
    disable_space_penalty: bool,
    #[clap(
        long = "space-penalty-rules",
        value_name = "JSON",
        help = "Use explicit left-space penalty rules instead of the dictionary's, e.g. '{\"rules\":[{\"pos\":[\"JKS\",\"JX\"],\"cost\":6000}]}'"
    )]
    space_penalty_rules: Option<String>,
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
        help = "Maximum cost difference from the best N-best result, over the whole line (e.g. 10000)"
    )]
    nbest_cost_threshold: Option<i64>,
    #[clap(
        help = "Input text file (default: stdin). Each line is tokenized on its own, with only its line terminator (\\n or \\r\\n) removed, so whitespace at the start or end of a line is handled as elsewhere in the line and offsets index the line"
    )]
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

/// Removes the line terminator that `BufRead::read_line` leaves at the end of
/// a line: a final `\n` and, only then, one `\r` before it (the rule of
/// `str::lines`).
///
/// Whitespace at the start or end of the line is kept, so it is tokenized
/// as elsewhere in the line (by default, whitespace of the `SPACE` category
/// is skipped and dropped by the segmenter, and U+3000 is a token with
/// IPADIC) and token offsets index the input line. A last line without `\n` is returned
/// unchanged, and any other `\r` stays part of the line. Unlike MeCab, which
/// removes only the `\n` and outputs the `\r` of a CRLF line as an unknown
/// word, the `\r` of a CRLF line is removed, so text with Windows line
/// endings does not end every line with a `\r` token.
///
/// # Arguments
///
/// * `line` - One line as read by `BufRead::read_line`.
///
/// # Returns
///
/// The line without its line terminator.
fn strip_line_terminator(line: &str) -> &str {
    match line.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').unwrap_or(line),
        None => line,
    }
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

    // Keep whitespace (default is to skip and drop it, as MeCab does)
    if args.keep_whitespace {
        builder.set_segmenter_keep_whitespace(true);
    }

    // Whitespace skipping in the lattice (default: enabled)
    if args.disable_skip_whitespace {
        builder.set_segmenter_skip_whitespace(false);
    }

    // Unknown-word grouping cap (default: unbounded)
    if let Some(max_grouping_len) = args.max_grouping_len {
        builder.set_segmenter_max_grouping_len(max_grouping_len);
    }

    // Unknown-word length ladder (default: enabled)
    if args.disable_unknown_word_ladder {
        builder.set_segmenter_unknown_word_ladder(false);
    }

    // Left-space penalty: on by default with the rules the dictionary ships
    // (ko-dic). `--disable-space-penalty` turns it off and
    // `--space-penalty-rules` replaces the rules (clap rejects both together).
    if args.disable_space_penalty {
        builder.set_segmenter_space_penalty_from_dictionary(false);
    }
    if let Some(rules) = args.space_penalty_rules.as_deref() {
        let config: SpacePenaltyConfig = serde_json::from_str(rules).map_err(|err| {
            LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("invalid --space-penalty-rules JSON: {err}"))
        })?;
        builder.set_segmenter_space_penalty(Some(&config));
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

        // Remove only the line terminator, not the whitespace at the ends of
        // the line, which is part of the input (#1115).
        let line = strip_line_terminator(&text);

        if nbest >= 2 {
            let results = worker.tokenize_nbest(line, nbest, nbest_unique, nbest_cost_threshold)?;
            for (rank, (tokens, cost)) in results.into_iter().enumerate() {
                writeln!(writer, "NBEST {} (cost={})", rank + 1, cost).map_err(io_err)?;
                write_output(&mut writer, output_format, tokens, &mut details_buf)?;
            }
        } else {
            let tokens = worker.tokenize(line)?;
            write_output(&mut writer, output_format, tokens, &mut details_buf)?;
        }
    }

    // Surface write errors instead of letting the implicit flush on drop
    // swallow them.
    writer.flush().map_err(io_err)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{TokenizeArgs, strip_line_terminator};

    /// `TokenizeArgs` is a `clap::Args` group; parse it through a throwaway
    /// command so the flag rules can be checked without the whole CLI.
    #[derive(Debug, Parser)]
    struct Cli {
        #[clap(flatten)]
        args: TokenizeArgs,
    }

    #[test]
    fn strip_line_terminator_removes_lf_and_crlf() {
        assert_eq!(strip_line_terminator("東京\n"), "東京");
        assert_eq!(strip_line_terminator("東京\r\n"), "東京");
        assert_eq!(strip_line_terminator("\n"), "");
        assert_eq!(strip_line_terminator("\r\n"), "");
    }

    #[test]
    fn strip_line_terminator_keeps_a_line_without_lf() {
        // The last line of the input may have no terminator; a `\r` that is
        // not followed by `\n` is part of the line.
        assert_eq!(strip_line_terminator("東京"), "東京");
        assert_eq!(strip_line_terminator(""), "");
        assert_eq!(strip_line_terminator("東京\r"), "東京\r");
        assert_eq!(strip_line_terminator("東\r京\n"), "東\r京");
    }

    #[test]
    fn strip_line_terminator_removes_one_cr_only() {
        assert_eq!(strip_line_terminator("東京\r\r\n"), "東京\r");
        assert_eq!(strip_line_terminator("東京\n\n"), "東京\n");
    }

    #[test]
    fn strip_line_terminator_keeps_whitespace_at_the_ends() {
        assert_eq!(strip_line_terminator("  東京 \n"), "  東京 ");
        assert_eq!(strip_line_terminator("\t東京\t\r\n"), "\t東京\t");
        assert_eq!(
            strip_line_terminator("\u{3000}東京\u{3000}\n"),
            "\u{3000}東京\u{3000}"
        );
        assert_eq!(
            strip_line_terminator("\u{a0}東京\u{a0}\n"),
            "\u{a0}東京\u{a0}"
        );
        assert_eq!(strip_line_terminator("   \n"), "   ");
    }

    #[test]
    fn disable_skip_whitespace_flag() {
        let base = ["lindera", "--dict", "embedded://ko-dic"];
        let cli = Cli::try_parse_from(base.iter()).unwrap();
        assert!(!cli.args.disable_skip_whitespace);
        let cli =
            Cli::try_parse_from(base.iter().chain(["--disable-skip-whitespace"].iter())).unwrap();
        assert!(cli.args.disable_skip_whitespace);
    }

    #[test]
    fn disable_space_penalty_conflicts_with_explicit_rules() {
        let rules = r#"{"rules":[{"pos":["JKS"],"cost":6000}]}"#;
        let base = ["lindera", "--dict", "embedded://ko-dic"];

        let cli =
            Cli::try_parse_from(base.iter().chain(["--disable-space-penalty"].iter())).unwrap();
        assert!(cli.args.disable_space_penalty);
        assert!(cli.args.space_penalty_rules.is_none());

        let cli = Cli::try_parse_from(base.iter().chain(["--space-penalty-rules", rules].iter()))
            .unwrap();
        assert!(!cli.args.disable_space_penalty);
        assert_eq!(cli.args.space_penalty_rules.as_deref(), Some(rules));

        let err = Cli::try_parse_from(
            base.iter()
                .chain(["--disable-space-penalty", "--space-penalty-rules", rules].iter()),
        )
        .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
