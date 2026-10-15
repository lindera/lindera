use std::path::PathBuf;

use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera_cli::get_version;

use super::io_err;

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "Export dictionary files from trained model",
    version = get_version(),
)]
pub struct ExportArgs {
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
        help = "Output directory (creates lex.csv, matrix.def, unk.def, char.def, feature.def, rewrite.def, left-id.def, right-id.def)"
    )]
    output: PathBuf,
    #[clap(
        long = "metadata",
        help = "Base metadata.json file to update with trained model values"
    )]
    metadata: Option<PathBuf>,
    #[clap(
        long = "cost-factor",
        help = "Override cost factor for weight-to-cost conversion (default: value from trained model, typically 700)"
    )]
    cost_factor: Option<i32>,
}

pub fn export(args: ExportArgs) -> LinderaResult<()> {
    use lindera::dictionary::trainer::SerializableModel;
    use std::fs::File;

    // Load trained model
    let model_file = File::open(&args.model).map_err(io_err)?;
    let mut model: SerializableModel =
        lindera::dictionary::trainer::model::Model::read_model(model_file)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))?;

    // Override cost factor if specified
    if let Some(cost_factor) = args.cost_factor {
        model.cost_factor = cost_factor;
    }

    // Write the dictionary source files, and metadata.json when a base
    // metadata.json is given; the bindings' export functions write the same
    // files with the same method (#1169).
    let files_created = model
        .export(&args.output, args.metadata.as_deref())
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    if args.metadata.is_some() {
        println!("Updated metadata.json with trained model values");
    }

    println!("Dictionary files exported to: {:?}", args.output);
    println!("Files created:");
    for file in &files_created {
        println!("  - {file:?}");
    }

    Ok(())
}
