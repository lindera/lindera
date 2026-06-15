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
        help = "Output directory (creates lex.csv, matrix.def, unk.def, char.def, feature.def, rewrite.def)"
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
    use std::fs::{self, File};

    // Load trained model
    let model_file = File::open(&args.model).map_err(io_err)?;
    let mut model: SerializableModel =
        lindera::dictionary::trainer::model::Model::read_model(model_file)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))?;

    // Override cost factor if specified
    if let Some(cost_factor) = args.cost_factor {
        model.cost_factor = cost_factor;
    }

    // Create output directory
    fs::create_dir_all(&args.output).map_err(io_err)?;

    // Export dictionary files
    let lexicon_path = args.output.join("lex.csv");
    let connector_path = args.output.join("matrix.def");
    let unk_path = args.output.join("unk.def");
    let char_def_path = args.output.join("char.def");
    let feature_def_path = args.output.join("feature.def");
    let rewrite_def_path = args.output.join("rewrite.def");

    // Write lexicon file using SerializableModel methods
    let mut lexicon_file = File::create(&lexicon_path).map_err(io_err)?;
    model
        .write_lexicon(&mut lexicon_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write connection matrix
    let mut connector_file = File::create(&connector_path).map_err(io_err)?;
    model
        .write_connection_costs(&mut connector_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write unknown word definitions
    let mut unk_file = File::create(&unk_path).map_err(io_err)?;
    model
        .write_unknown_dictionary(&mut unk_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write character definition file
    let mut char_def_file = File::create(&char_def_path).map_err(io_err)?;
    model
        .write_char_def(&mut char_def_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write feature definition file
    let mut feature_def_file = File::create(&feature_def_path).map_err(io_err)?;
    model
        .write_feature_def(&mut feature_def_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write rewrite rule definition file
    let mut rewrite_def_file = File::create(&rewrite_def_path).map_err(io_err)?;
    model
        .write_rewrite_def(&mut rewrite_def_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write left-id.def
    let left_id_path = args.output.join("left-id.def");
    let mut left_id_file = File::create(&left_id_path).map_err(io_err)?;
    model
        .write_left_id_def(&mut left_id_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Write right-id.def
    let right_id_path = args.output.join("right-id.def");
    let mut right_id_file = File::create(&right_id_path).map_err(io_err)?;
    model
        .write_right_id_def(&mut right_id_file)
        .map_err(|err| LinderaErrorKind::Io.with_error(err))?;

    // Handle metadata.json update if provided
    let mut files_created = vec![
        lexicon_path.clone(),
        connector_path.clone(),
        unk_path.clone(),
        char_def_path.clone(),
        feature_def_path.clone(),
        rewrite_def_path.clone(),
        left_id_path.clone(),
        right_id_path.clone(),
    ];

    if let Some(metadata_path) = &args.metadata {
        let output_metadata_path = args.output.join("metadata.json");
        let mut metadata_file = File::create(&output_metadata_path).map_err(io_err)?;

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
