use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};

use lindera_ipadic_builder::builder::build;
use lindera_ipadic_builder::error::BuildResult;

fn main() -> BuildResult<()> {
    let app = App::new(crate_name!())
        .setting(AppSettings::DeriveDisplayOrder)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .help_message("Prints help information.")
        .version_message("Prints version information.")
        .version_short("v")
        .arg(
            Arg::with_name("INPUT_DIR")
                .help("The directory where the IPADIC source containing.")
                .value_name("INPUT_DIR")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("OUTPUT_DIR")
                .help("The directory where the IPADIC binary for Lindera is output.")
                .value_name("OUTPUT_DIR")
                .required(true)
                .takes_value(true),
        );

    let matches = app.get_matches();

    let input_dir = matches.value_of("INPUT_DIR").unwrap();
    let output_dir = matches.value_of("OUTPUT_DIR").unwrap();

    match build(input_dir, output_dir) {
        Ok(()) => println!("{}", "done"),
        Err(msg) => println!("{}", msg),
    }

    Ok(())
}
