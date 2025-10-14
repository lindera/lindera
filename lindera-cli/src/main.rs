use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::{Parser, Subcommand};

use lindera::LinderaResult;
use lindera::character_filter::CharacterFilterLoader;
use lindera::dictionary::{DictionaryBuilder, DictionaryKind, Metadata};
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
    #[cfg(feature = "train")]
    Train(TrainArgs),
    #[cfg(feature = "train")]
    Export(ExportArgs),
}

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "List embedded morphological analysis dictionaries",
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
    #[clap(help = "Input text file (default: stdin)")]
    input_file: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
#[clap(author,
    about = "Build a morphological analysis dictionary",
    version = get_version(),
)]
struct BuildArgs {
    #[clap(
        short = 's',
        long = "src",
        required = true,
        help = "Source directory containing dictionary CSV files"
    )]
    src: PathBuf,
    #[clap(
        short = 'd',
        long = "dest",
        required = true,
        help = "Destination directory for compiled dictionary"
    )]
    dest: PathBuf,
    #[clap(
        short = 'm',
        long = "metadata",
        required = true,
        help = "Metadata configuration file (metadata.json)"
    )]
    metadata: PathBuf,
    #[clap(
        short = 'u',
        long = "user",
        help = "Build user dictionary (default: system dictionary)"
    )]
    user: bool,
}

#[cfg(feature = "train")]
#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Train a morphological analysis model from corpus",
    version = get_version(),
)]
struct TrainArgs {
    #[clap(
        short = 's',
        long = "seed",
        required = true,
        help = "Seed lexicon file (CSV format) to be weighted"
    )]
    seed: PathBuf,
    #[clap(
        short = 'c',
        long = "corpus",
        required = true,
        help = "Training corpus (annotated text)"
    )]
    corpus: PathBuf,
    #[clap(
        short = 'C',
        long = "char-def",
        required = true,
        help = "Character definition file (char.def)"
    )]
    char_def: PathBuf,
    #[clap(
        short = 'u',
        long = "unk-def",
        required = true,
        help = "Unknown word definition file (unk.def) to be weighted"
    )]
    unk_def: PathBuf,
    #[clap(
        short = 'f',
        long = "feature-def",
        required = true,
        help = "Feature definition file (feature.def)"
    )]
    feature_def: PathBuf,
    #[clap(
        short = 'r',
        long = "rewrite-def",
        required = true,
        help = "Rewrite rule definition file (rewrite.def)"
    )]
    rewrite_def: PathBuf,
    #[clap(
        short = 'o',
        long = "output",
        required = true,
        help = "Output model file"
    )]
    output: PathBuf,
    #[clap(
        short = 'l',
        long = "lambda",
        default_value = "0.01",
        help = "L1 regularization (0.0-1.0)"
    )]
    lambda: f64,
    #[clap(
        short = 'i',
        long = "max-iterations",
        default_value = "100",
        help = "Maximum number of iterations for training"
    )]
    iter: u64,
    #[clap(
        short = 't',
        long = "max-threads",
        help = "Maximum number of threads (defaults to CPU core count, auto-adjusted based on dataset size)"
    )]
    max_threads: Option<usize>,
}

#[cfg(feature = "train")]
#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Export dictionary files from trained model",
    version = get_version(),
)]
struct ExportArgs {
    #[clap(
        short = 'm',
        long = "model",
        required = true,
        help = "Trained model file (.dat format)"
    )]
    model: PathBuf,
    #[clap(
        short = 'o',
        long = "output",
        required = true,
        help = "Output directory (creates lex.csv, matrix.def, unk.def, char.def)"
    )]
    output: PathBuf,
    #[clap(
        long = "metadata",
        help = "Base metadata.json file to update with trained model values"
    )]
    metadata: Option<PathBuf>,
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

fn main() -> LinderaResult<()> {
    let args = Args::parse();

    match args.command {
        Commands::List(args) => list(args),
        Commands::Tokenize(args) => tokenize(args),
        Commands::Build(args) => build(args),
        #[cfg(feature = "train")]
        Commands::Train(args) => train(args),
        #[cfg(feature = "train")]
        Commands::Export(args) => export(args),
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

fn tokenize(args: TokenizeArgs) -> LinderaResult<()> {
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
    let metadata: Metadata = serde_json::from_reader(
        File::open(&args.metadata)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
    )
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    let builder = DictionaryBuilder::new(metadata);

    if args.user {
        let output_file = if let Some(filename) = args.src.file_name() {
            let mut output_file = Path::new(&args.dest).join(filename);
            output_file.set_extension("bin");
            output_file
        } else {
            return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!("failed to get filename")));
        };
        builder.build_user_dictionary(&args.src, &output_file)
    } else {
        builder.build_dictionary(&args.src, &args.dest)
    }
}

#[cfg(feature = "train")]
fn train(args: TrainArgs) -> LinderaResult<()> {
    use lindera::dictionary::trainer::{Corpus, Trainer, TrainerConfig};
    use std::fs::File;

    // Load configuration
    let config = TrainerConfig::from_paths(
        &args.seed,
        &args.char_def,
        &args.unk_def,
        &args.feature_def,
        &args.rewrite_def,
    )
    .map_err(|err| LinderaErrorKind::Args.with_error(err))?;

    // Initialize trainer
    let trainer = Trainer::new(config)
        .map_err(|err| LinderaErrorKind::Args.with_error(err))?
        .regularization_cost(args.lambda)
        .max_iter(args.iter)
        .num_threads(args.max_threads.unwrap_or_else(num_cpus::get));

    // Load corpus
    let corpus_file = File::open(&args.corpus)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    let corpus =
        Corpus::from_reader(corpus_file).map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    println!("Training with {} examples...", corpus.len());

    // Train model
    let model = trainer
        .train(corpus)
        .map_err(|err| LinderaErrorKind::Args.with_error(err))?;

    // Save model
    // Create parent directory if it doesn't exist
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    }
    let mut output_file = File::create(&args.output)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    model
        .write_model(&mut output_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    println!("Model saved to {:?}", args.output);
    Ok(())
}

#[cfg(feature = "train")]
fn export(args: ExportArgs) -> LinderaResult<()> {
    use lindera::dictionary::trainer::SerializableModel;
    use std::fs::{self, File};
    use std::io::Write;

    // Load trained model
    let model_file = File::open(&args.model)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    let model: SerializableModel =
        lindera::dictionary::trainer::model::Model::read_model(model_file)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))?;

    // Create output directory
    fs::create_dir_all(&args.output)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    // Export dictionary files
    let lexicon_path = args.output.join("lex.csv");
    let connector_path = args.output.join("matrix.def");
    let unk_path = args.output.join("unk.def");
    let char_def_path = args.output.join("char.def");

    // Write lexicon file using SerializableModel methods
    let mut lexicon_file = File::create(&lexicon_path)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    model
        .write_lexicon(&mut lexicon_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write connection matrix
    let mut connector_file = File::create(&connector_path)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    model
        .write_connection_costs(&mut connector_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write unknown word definitions
    let mut unk_file = File::create(&unk_path)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    model
        .write_unknown_dictionary(&mut unk_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write character definition file
    let mut char_def_file = File::create(&char_def_path)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    writeln!(
        char_def_file,
        "# Character definition file generated from trained model"
    )
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "# Format: CATEGORY_NAME invoke group length")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "DEFAULT 0 1 0")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "HIRAGANA 1 1 0")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "KATAKANA 1 1 0")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "KANJI 0 0 2")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "ALPHA 1 1 0")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "NUMERIC 1 1 0")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file).map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    writeln!(char_def_file, "# Character mappings")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "0x3041..0x3096 HIRAGANA  # Hiragana")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "0x30A1..0x30F6 KATAKANA  # Katakana")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(
        char_def_file,
        "0x4E00..0x9FAF KANJI     # CJK Unified Ideographs"
    )
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "0x0030..0x0039 NUMERIC   # ASCII Digits")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "0x0041..0x005A ALPHA     # ASCII Uppercase")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    writeln!(char_def_file, "0x0061..0x007A ALPHA     # ASCII Lowercase")
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    // Handle metadata.json update if provided
    let mut files_created = vec![
        lexicon_path.clone(),
        connector_path.clone(),
        unk_path.clone(),
        char_def_path.clone(),
    ];

    if let Some(metadata_path) = &args.metadata {
        let output_metadata_path = args.output.join("metadata.json");
        let mut metadata_file = File::create(&output_metadata_path)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        model
            .update_metadata_json(metadata_path, &mut metadata_file)
            .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

        files_created.push(output_metadata_path);
        println!("Updated metadata.json with trained model values");
    }

    println!("Dictionary files exported to: {:?}", args.output);
    println!("Files created:");
    for file in &files_created {
        println!("  - {file:?}");
    }

    Ok(())
}
