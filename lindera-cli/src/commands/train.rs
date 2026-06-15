use std::path::PathBuf;

use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera_cli::get_version;

use super::io_err;

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Train a morphological analysis model from corpus",
    version = get_version(),
)]
pub struct TrainArgs {
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
        help = "Regularization coefficient (0.0-1.0)"
    )]
    lambda: f64,
    #[clap(
        short = 'R',
        long = "regularization",
        default_value = "l1",
        help = "Regularization type: l1, l2, or elasticnet"
    )]
    regularization: String,
    #[clap(
        long = "elastic-net-l1-ratio",
        default_value = "0.5",
        help = "L1 ratio for Elastic Net regularization (0.0-1.0, only used with --regularization elasticnet)"
    )]
    elastic_net_l1_ratio: f64,
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

pub fn train(args: TrainArgs) -> LinderaResult<()> {
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

    // Parse regularization type and initialize trainer
    let mut trainer = Trainer::new(config)
        .map_err(|err| LinderaErrorKind::Args.with_error(err))?
        .regularization_cost(args.lambda)
        .max_iter(args.iter)
        .num_threads(args.max_threads.unwrap_or_else(num_cpus::get));

    match args.regularization.to_lowercase().as_str() {
        "l1" => {}
        "l2" => {
            trainer = trainer.use_l2(true);
        }
        "elasticnet" | "elastic_net" | "elastic-net" => {
            trainer = trainer.elastic_net_l1_ratio(args.elastic_net_l1_ratio);
        }
        _ => {
            return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                "regularization must be 'l1', 'l2', or 'elasticnet'"
            )));
        }
    };

    // Load corpus
    let corpus_file = File::open(&args.corpus).map_err(io_err)?;
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
        std::fs::create_dir_all(parent).map_err(io_err)?;
    }
    let mut output_file = File::create(&args.output).map_err(io_err)?;
    model
        .write_model(&mut output_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    println!("Model saved to {:?}", args.output);
    Ok(())
}
