use std::path::Path;

use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg};

use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::LinderaResult;
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;
use lindera_core::error::LinderaErrorKind;

fn main() -> LinderaResult<()> {
    let app = App::new(crate_name!())
        .setting(AppSettings::DeriveDisplayOrder)
        .version(crate_version!())
        .about(crate_description!())
        .help_message("Prints help information.")
        .version_message("Prints version information.")
        .version_short("v")
        .arg(
            Arg::with_name("DICT_SRC")
                .help("The dictionary source path.")
                .short("s")
                .long("dict-src")
                .value_name("DICT_SRC")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("DICT_DEST")
                .help("The dictionary destination path.")
                .short("d")
                .long("dict-dest")
                .value_name("DICT_DEST")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("USER_DICT_SRC")
                .help("The user dictionary source path.")
                .short("S")
                .long("user-dict-src")
                .value_name("USER_DICT_SRC")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("USER_DICT_DEST")
                .help("The user dictionary destination path.")
                .short("D")
                .long("user-dict-dest")
                .value_name("USER_DICT_DEST")
                .takes_value(true),
        );

    let matches = app.get_matches();

    let dict_builder = IpadicBuilder::new();

    if matches.is_present("DICT_SRC") {
        if matches.is_present("DICT_DEST") {
            let dict_src = matches.value_of("DICT_SRC").unwrap();
            let dict_dest = matches.value_of("DICT_DEST").unwrap();
            match dict_builder.build_dictionary(Path::new(dict_src), Path::new(dict_dest)) {
                Ok(()) => (),
                Err(msg) => {
                    return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(msg)));
                },
            }
        } else {
            return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!("--dict-dest is required")));
        }
    }

    if matches.is_present("USER_DICT_SRC") {
        if matches.is_present("USER_DICT_DEST") {
            let user_dict_src = matches.value_of("USER_DICT_SRC").unwrap();
            let user_dict_dest = matches.value_of("USER_DICT_DEST").unwrap();
            match dict_builder
                .build_user_dictionary(Path::new(user_dict_src), Path::new(user_dict_dest))
            {
                Ok(()) => (),
                Err(msg) => {
                    return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(msg)));
                },
            }
        } else {
            return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!("--user-dict-dest is required")));
        }
    }

    Ok(())
}
