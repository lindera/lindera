#[macro_use]
extern crate clap;

use clap::{App, AppSettings};
use std::io;

use lindera::Tokenizer;

fn main() {
    let app = App::new(crate_name!())
        .setting(AppSettings::DeriveDisplayOrder)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .help_message("Prints help information.")
        .version_message("Prints version information.")
        .version_short("v");

    let _matches = app.get_matches();

    let mut tokenizer = Tokenizer::normal();
    loop {
        let mut text = String::new();
        io::stdin().read_line(&mut text).expect("Failed to read line");
        text = text.trim().to_string();

        for token in tokenizer.tokenize(&text) {
            println!("{}\t{}", token.text,token.detail.reading);
        }
        println!("EOS")
    }
}
