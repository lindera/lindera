use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};

use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::error::LinderaErrorKind;
use lindera_core::user_dictionary::UserDictionary;
use lindera_core::LinderaResult;
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;

fn write_binary_data(file_path: &Path, user_dict: &UserDictionary) -> LinderaResult<()> {
    let mut wtr = io::BufWriter::new(
        File::create(file_path)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
    );
    bincode::serialize_into(&mut wtr, user_dict)
        .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
    wtr.flush()
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    Ok(())
}

fn main() -> LinderaResult<()> {
    let app = App::new(crate_name!())
        .setting(AppSettings::DeriveDisplayOrder)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .help_message("Prints help information.")
        .version_message("Prints version information.")
        .version_short("v")
        .arg(
            Arg::with_name("INPUT_CSV")
                .help("CSV file of UserDictionary.")
                .value_name("INPUT_CSV")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("OUTPUT_FILE")
                .help("Binary file of the UserDictionary.")
                .value_name("OUTPUT_FILE")
                .required(true)
                .takes_value(true),
        );

    let matches = app.get_matches();

    let input_csv_path = Path::new(matches.value_of("INPUT_CSV").unwrap()).to_path_buf();
    let output_file_path = Path::new(matches.value_of("OUTPUT_FILE").unwrap()).to_path_buf();

    let builder = IpadicBuilder::new();
    match builder.build_user_dict(&input_csv_path) {
        Ok(user_dict) => {
            write_binary_data(&output_file_path, &user_dict)?;
            println!("done");
        }
        Err(msg) => println!("{}!", msg),
    }

    Ok(())
}
