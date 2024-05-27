use std::error::Error;

#[cfg(feature = "cc-cedict")]
fn main() -> Result<(), Box<dyn Error>> {
    use std::env;
    use std::fs::{create_dir, rename, File};
    use std::io::{self, Cursor, Read, Write};
    use std::path::Path;

    use encoding::all::UTF_8;
    use encoding::{EncoderTrap, Encoding};
    use flate2::read::GzDecoder;
    use tar::Archive;

    use lindera_cc_cedict_builder::cc_cedict_builder::CcCedictBuilder;
    use lindera_core::dictionary_builder::DictionaryBuilder;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Directory path for build package
    let build_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    // Dictionary file name
    let file_name = "CC-CEDICT-MeCab-0.1.0-20200409.tar.gz";

    // CC-CEDICT MeCab directory
    let input_dir = Path::new(&build_dir).join("CC-CEDICT-MeCab-0.1.0-20200409");

    // Lindera CC-CEDICT directory
    let output_dir = Path::new(&build_dir).join("lindera-cc-cedict");

    if std::env::var("DOCS_RS").is_ok() {
        // Create directory for dummy input directory for build docs
        create_dir(&input_dir)?;

        // Create dummy char.def
        let mut dummy_char_def = File::create(input_dir.join("char.def"))?;
        dummy_char_def.write_all(b"DEFAULT 0 1 0\n")?;

        // Create dummy CSV file
        let mut dummy_dict_csv = File::create(input_dir.join("dummy_dict.csv"))?;
        dummy_dict_csv
            .write_all(
                &UTF_8
                    .encode(
                        "测试,0,0,-1131,*,*,*,*,ce4 shi4,測試,测试,to test (machinery etc)/to test (students)/test/quiz/exam/beta (software)/\n",
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
            let download_url =
                "https://dlwqk3ibdg1xh.cloudfront.net/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz";
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
    let builder = CcCedictBuilder::new();
    builder.build_dictionary(&input_dir, &output_dir)?;

    Ok(())
}

#[cfg(not(feature = "cc-cedict"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}
