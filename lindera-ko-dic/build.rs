use std::env;
use std::error::Error;
use std::fs::{create_dir, rename};
use std::io::Cursor;
use std::path::Path;

use encoding::all::UTF_8;
use encoding::{EncoderTrap, Encoding};
use flate2::read::GzDecoder;
use tar::Archive;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_ko_dic_builder::ko_dic_builder::KodicBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Directory path for build package
    let build_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    // Dictionary file name
    let file_name = "mecab-ko-dic-2.1.1-20180720.tar.gz";

    // UniDic MeCab directory
    let input_dir = Path::new(&build_dir).join("mecab-ko-dic-2.1.1-20180720");

    // Lindera IPADIC directory
    let output_dir = Path::new(&build_dir).join("lindera-ko-dic");

    if std::env::var("DOCS_RS").is_ok() {
        // Use dummy data in docs.rs.
        create_dir(&input_dir)?;

        let mut dummy_char_def = File::create(input_dir.join("char.def")).await?;
        dummy_char_def.write_all(b"DEFAULT 0 1 0\n").await?;

        let mut dummy_dict_csv = File::create(input_dir.join("dummy_dict.csv")).await?;
        dummy_dict_csv
            .write_all(
                &UTF_8
                    .encode(
                        "테스트,1785,3543,4721,NNG,행위,F,테스트,*,*,*,*\n",
                        EncoderTrap::Ignore,
                    )
                    .unwrap(),
            )
            .await?;

        File::create(input_dir.join("unk.def")).await?;
        let mut dummy_matrix_def = File::create(input_dir.join("matrix.def")).await?;
        dummy_matrix_def.write_all(b"0 1 0\n").await?;
    } else {
        // Source file path for build package
        let source_path_for_build = Path::new(&build_dir).join(&file_name);

        // Download source file to build directory
        if !source_path_for_build.exists() {
            // copy(&source_path, &source_path_for_build)?;
            let tmp_path = Path::new(&build_dir).join(file_name.to_owned() + ".download");

            // Download a tarball
            let download_url =
                "https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/mecab-ko-dic-2.1.1-20180720.tar.gz";
            let mut resp = reqwest::get(download_url).await?;

            // Save a ttarball
            let mut dest = File::create(&tmp_path).await?;
            while let Some(chunk) = resp.chunk().await? {
                dest.write_all(&chunk).await?;
            }
            rename(tmp_path, &source_path_for_build).expect("Failed to rename temporary file");
        }

        // Decompress a tarball
        let mut tar_gz = File::open(&source_path_for_build).await?;
        let mut buffer = Vec::new();
        tar_gz.read_to_end(&mut buffer).await?;
        let cursor = Cursor::new(buffer);
        let gzdecoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(gzdecoder);
        archive.unpack(&build_dir)?;
    }

    // Build a dictionary
    let builder = KodicBuilder::new();
    builder.build_dictionary(&input_dir, &output_dir)?;

    Ok(())
}
