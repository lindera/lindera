use std::env;
use std::error::Error;
use std::fs::{copy, create_dir};
use std::io::Cursor;
use std::path::Path;

use encoding::all::EUC_JP;
use encoding::{EncoderTrap, Encoding};
use flate2::read::GzDecoder;
use tar::Archive;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Directory path for build package
    let build_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    // Resources directory path
    let resources_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("ipadic");

    // Dictionary file name
    let file_name = "mecab-ipadic-2.7.0-20070801.tar.gz";
    // Source file path
    let source_path = Path::new(&resources_dir).join(file_name);

    // MeCab IPADIC directory
    let input_dir = Path::new(&build_dir).join("mecab-ipadic-2.7.0-20070801");

    // Lindera IPADIC directory
    let output_dir = Path::new(&build_dir).join("lindera-ipadic");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    if std::env::var("DOCS_RS").is_ok() {
        // Use dummy data in docs.rs.
        create_dir(&input_dir)?;

        let mut dummy_char_def = File::create(input_dir.join("char.def")).await?;
        dummy_char_def.write_all(b"DEFAULT 0 1 0\n").await?;

        let mut dummy_dict_csv = File::create(input_dir.join("dummy_dict.csv")).await?;
        dummy_dict_csv
            .write_all(
                &EUC_JP
                    .encode(
                        "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
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

        // Copy source file to build directory
        if !source_path_for_build.exists() {
            copy(&source_path, &source_path_for_build)?;
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
    let builder = IpadicBuilder::new();
    builder.build_dictionary(&input_dir, &output_dir)?;

    Ok(())
}
