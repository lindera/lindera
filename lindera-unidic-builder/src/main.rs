use std::path::PathBuf;

use clap::{AppSettings, Parser};

use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;
use lindera_unidic_builder::unidic_builder::UnidicBuilder;

/// Lindera UniDic dictionary builder
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None, setting = AppSettings::DeriveDisplayOrder)]
struct Args {
    /// The dictionary source directory.
    #[clap(short = 's', long = "dict-src", value_name = "DICT_SRC")]
    dict_src: Option<PathBuf>,

    /// The dictionary destination directory.
    #[clap(short = 'd', long = "dict-dest", value_name = "DICT_DEST")]
    dict_dest: Option<PathBuf>,
}

fn main() -> LinderaResult<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let dict_builder = UnidicBuilder::new();

    if args.dict_src.is_some() {
        if args.dict_dest.is_some() {
            let dict_src = args.dict_src.unwrap();
            let dict_dest = args.dict_dest.unwrap();
            match dict_builder.build_dictionary(&dict_src, &dict_dest) {
                Ok(()) => (),
                Err(msg) => {
                    return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(msg)));
                }
            }
        } else {
            return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                "`--dict-dest` is required when `--dict-src` is specified"
            )));
        }
    }

    Ok(())
}
