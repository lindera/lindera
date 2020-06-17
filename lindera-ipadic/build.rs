use std::env;
use std::error::Error;
use std::fs::File;
use std::path::Path;

use flate2::read::GzDecoder;
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

    let lindera_ipadic_builder_ver = "0.4.1";
    let ipadic_ver = "2.7.0-20070801";
    let lindera_ipadic_archive_file_name = format!("lindera-ipadic-{}.tar.gz", ipadic_ver);
    let download_url = format!("https://github.com/lindera-morphology/lindera-ipadic-builder/releases/download/v{}/{}", lindera_ipadic_builder_ver, lindera_ipadic_archive_file_name);

    // Download a tarball
    let mut resp = reqwest::get(&download_url).await.unwrap();

    // Save a tarball
    let dest_path = Path::new(&out_dir).join(lindera_ipadic_archive_file_name);
    let mut dest = TokioFile::create(&dest_path).await.unwrap();
    while let Some(chunk) = resp.chunk().await.unwrap() {
        dest.write_all(&chunk).await?;
    }

    // Decompress a tarball
    let tar_gz = File::open(&dest_path).unwrap();
    let gzdecoder = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(gzdecoder);
    archive.unpack(&out_dir).unwrap();

    Ok(())
}
