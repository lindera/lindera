use std::env;
use std::error::Error;
use std::fs::File;
use std::path::Path;

use flate2::read::GzDecoder;
use lindera_ipadic_builder::build;
use reqwest;
use tar::Archive;
use tokio;
use tokio::fs::File as TokioFile;
use tokio::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=lindera-ipadic");

    let ipadic_ver = "2.7.0-20070801";
    let file_name = format!("mecab-ipadic-{}.tar.gz", ipadic_ver);

    // Download a tarball
    let download_url = format!(
        "http://jaist.dl.sourceforge.net/project/mecab/mecab-ipadic/{}/{}",
        ipadic_ver, file_name
    );
    let mut resp = reqwest::get(&download_url).await.unwrap();

    // Save a ttarball
    let dest_path = Path::new(&out_dir).join(file_name);
    let mut dest = TokioFile::create(&dest_path).await.unwrap();
    while let Some(chunk) = resp.chunk().await.unwrap() {
        dest.write_all(&chunk).await?;
    }

    // Decompress a tarball
    let tar_gz = File::open(&dest_path).unwrap();
    let gzdecoder = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(gzdecoder);
    archive.unpack(&out_dir).unwrap();

    // Build dictionary
    let input_dir = Path::new(&out_dir).join(format!("mecab-ipadic-{}", ipadic_ver));
    let output_dir = "./lindera-ipadic";
    build(&input_dir.to_str().unwrap(), output_dir).unwrap();

    Ok(())
}
