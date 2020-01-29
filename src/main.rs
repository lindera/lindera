#[macro_use]
extern crate clap;

use std::io::Write;

use clap::{App, AppSettings, Arg, SubCommand};

use lindera::cmd::build::run_build_cli;
use lindera::cmd::tokenize::run_tokenize_cli;

fn main() {
    let app = App::new(crate_name!())
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .help_message("Prints help information.")
        .version_message("Prints version information.")
        .version_short("v")
        .subcommand(
            SubCommand::with_name("tokenize")
                .name("tokenize")
                .setting(AppSettings::DeriveDisplayOrder)
                .version(crate_version!())
                .author(crate_authors!())
                .about("The `lindera tokenize` CLI tokenizes the text.")
                .help_message("Prints help information.")
                .version_message("Prints version information.")
                .version_short("v")
                .arg(
                    Arg::with_name("MODE")
                        .help("The tokenization mode. `normal` or` search` can be specified. If not specified, use the default mode.")
                        .short("m")
                        .long("mode")
                        .value_name("MODE")
                        .default_value("normal")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("OUTPUT")
                        .help("The output format. `mecab`, `wakati` or `json` can be specified. If not specified, use the default output format.")
                        .short("o")
                        .long("output")
                        .value_name("OUTPUT")
                        .default_value("mecab")
                        .takes_value(true),
                )
        )
        .subcommand(
            SubCommand::with_name("build")
                .name("build")
                .setting(AppSettings::DeriveDisplayOrder)
                .version(crate_version!())
                .author(crate_authors!())
                .about("The `lindera build` CLI builds the dictionary.")
                .help_message("Prints help information.")
                .version_message("Prints version information.")
                .version_short("v")
        )
        .get_matches();

    let (subcommand, some_options) = app.subcommand();
    let options = some_options.unwrap();
    let run_cli = match subcommand {
        "tokenize" => run_tokenize_cli,
        "build" => run_build_cli,
        _ => panic!("Subcommand {} is unknown", subcommand),
    };

    if let Err(ref e) = run_cli(options) {
        let stderr = &mut std::io::stderr();
        let errmsg = "Error writing to stderr";
        writeln!(stderr, "{}", e).expect(errmsg);
        std::process::exit(1);
    }
}
