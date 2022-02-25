use std::path::PathBuf;

use clap::{AppSettings, Parser};

use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;

/// Lindera IPADIC builder CLI
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None, setting = AppSettings::DeriveDisplayOrder)]
struct Args {
    /// The dictionary source directory.
    #[clap(short = 's', long = "dict-src", value_name = "DICT_SRC")]
    dict_src: Option<PathBuf>,

    /// The dictionary destination directory.
    #[clap(short = 'd', long = "dict-dest", value_name = "DICT_DEST")]
    dict_dest: Option<PathBuf>,

    /// The user dictionary source file.
    #[clap(short = 'S', long = "user-dict-src", value_name = "USER_DICT_SRC")]
    user_dict_src: Option<PathBuf>,

    /// The user dictionary destination file.
    #[clap(short = 'D', long = "user-dict-dest", value_name = "USER_DICT_DEST")]
    user_dict_dest: Option<PathBuf>,
}

fn main() -> LinderaResult<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let dict_builder = IpadicBuilder::new();

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

    if args.user_dict_src.is_some() {
        if args.user_dict_dest.is_some() {
            let user_dict_src = args.user_dict_src.unwrap();
            let user_dict_dest = args.user_dict_dest.unwrap();
            match dict_builder.build_user_dictionary(&user_dict_src, &user_dict_dest) {
                Ok(()) => (),
                Err(msg) => {
                    return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(msg)));
                }
            }
        } else {
            return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                "`--user-dict-dest` is required when `--user-dict-src` is specified"
            )));
        }
    }

    Ok(())
}
