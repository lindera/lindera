use std::fs::File;
use std::path::{Path, PathBuf};

use lindera::LinderaResult;
use lindera::dictionary::{DictionaryBuilder, Metadata};
use lindera::error::LinderaErrorKind;
use lindera_cli::get_version;

use super::io_err;

#[derive(Debug, clap::Args)]
#[clap(author,
    about = "Build a morphological analysis dictionary",
    version = get_version(),
)]
pub struct BuildArgs {
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
    #[clap(
        short = 'f',
        long = "context-id-freq",
        help = "Context-ID access-frequency file used to order connection-cost IDs (requires connection_id_mapping in metadata)"
    )]
    context_id_freq: Option<PathBuf>,
}

pub fn build(args: BuildArgs) -> LinderaResult<()> {
    let metadata: Metadata =
        serde_json::from_reader(File::open(&args.metadata).map_err(io_err)?).map_err(io_err)?;

    let mut builder = DictionaryBuilder::new(metadata);
    if let Some(freq) = &args.context_id_freq {
        builder = builder.with_context_id_freq(freq.clone());
    }

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
