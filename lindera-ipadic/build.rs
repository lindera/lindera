use std::error::Error;

#[cfg(feature = "ipadic")]
fn main() -> Result<(), Box<dyn Error>> {
    use std::env;
    use std::fs::{rename, create_dir, File};
    use std::io::{self, Cursor, Read, Write};
    use std::path::Path;

    use encoding::all::EUC_JP;
    use encoding::{EncoderTrap, Encoding};
    use flate2::read::GzDecoder;
    use tar::Archive;

    use lindera_core::dictionary_builder::DictionaryBuilder;
    use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Directory path for build package
    let build_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    // Dictionary file name
    let file_name = "mecab-ipadic-2.7.0-20070801.tar.gz";

    // MeCab IPADIC directory
    let input_dir = Path::new(&build_dir).join("mecab-ipadic-2.7.0-20070801");

    // Lindera IPADIC directory
    let output_dir = Path::new(&build_dir).join("lindera-ipadic");

    if std::env::var("DOCS_RS").is_ok() {
        // Create directory for dummy input directory for build docs
        create_dir(&input_dir)?;

        // Create dummy char.def
        let mut dummy_char_def = File::create(input_dir.join("char.def"))?;
        dummy_char_def.write_all(b"DEFAULT 0 1 0\n")?;

        // Create dummy CSV file
        let mut dummy_dict_csv = File::create(input_dir.join("dummy_dict.csv"))?;
        dummy_dict_csv.write_all(
            &EUC_JP
                .encode(
                    "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
                    EncoderTrap::Ignore,
                )
                .unwrap(),
        )?;

        // Create dummy unk.def
        File::create(input_dir.join("unk.def"))?;
        let mut dummy_matrix_def = File::create(input_dir.join("matrix.def"))?;
        dummy_matrix_def.write_all(b"0 1 0\n")?;
    } else {
        // Source file path for build package
        let source_path_for_build = Path::new(&build_dir).join(file_name);

        // Download source file to build directory
        if !source_path_for_build.exists() {
            // copy(&source_path, &source_path_for_build)?;
            let tmp_path = Path::new(&build_dir).join(file_name.to_owned() + ".download");

            // Download a tarball
            let download_url = "https://github.com/lindera-morphology/mecab-ipadic/archive/refs/tags/2.7.0-20070801.tar.gz";
            let resp = ureq::get(download_url).call()?;
            let mut dest = File::create(&tmp_path)?;

            io::copy(&mut resp.into_reader(), &mut dest)?;
            dest.flush()?;

            rename(tmp_path, &source_path_for_build).expect("Failed to rename temporary file");
        }

        // Decompress a tar.gz file
        let mut tar_gz = File::open(source_path_for_build)?;
        let mut buffer = Vec::new();
        tar_gz.read_to_end(&mut buffer)?;
        let cursor = Cursor::new(buffer);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);
        archive.unpack(&build_dir)?;
    }

    // Build a dictionary
    let builder = IpadicBuilder::new();
    builder.build_dictionary(&input_dir, &output_dir)?;

    Ok(())
}

#[cfg(not(feature = "ipadic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}
