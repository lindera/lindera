use std::error::Error;

#[cfg(feature = "ko-dic")]
fn main() -> Result<(), Box<dyn Error>> {
    use std::env;
    use std::fs::{create_dir, File};
    use std::io::{Cursor, Read, Write};
    use std::path::Path;

    use encoding::all::UTF_8;
    use encoding::{EncoderTrap, Encoding};
    use flate2::read::GzDecoder;
    use tar::Archive;

    use lindera_core::dictionary_builder::DictionaryBuilder;
    use lindera_ko_dic_builder::ko_dic_builder::KoDicBuilder;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Directory path for build package
    let build_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    // UniDic MeCab directory
    let input_dir = Path::new(&build_dir).join("mecab-ko-dic-2.1.1-20180720");

    if std::env::var("DOCS_RS").is_ok() {
        // Use dummy data in docs.rs.
        create_dir(&input_dir)?;

        let mut dummy_char_def = File::create(input_dir.join("char.def"))?;
        dummy_char_def.write_all(b"DEFAULT 0 1 0\n")?;

        let mut dummy_dict_csv = File::create(input_dir.join("dummy_dict.csv"))?;
        dummy_dict_csv.write_all(
            &UTF_8
                .encode(
                    "테스트,1785,3543,4721,NNG,행위,F,테스트,*,*,*,*\n",
                    EncoderTrap::Ignore,
                )
                .unwrap(),
        )?;

        File::create(input_dir.join("unk.def"))?;
        let mut dummy_matrix_def = File::create(input_dir.join("matrix.def"))?;
        dummy_matrix_def.write_all(b"0 1 0\n")?;
    } else {
        // Resources directory
        let resources_dir_path = Path::new("resources");

        // Dictionary file name
        let dict_file_name = "mecab-ko-dic-2.1.1-20180720.tar.gz";

        // Source dictionary file path
        let source_dict_file_path = resources_dir_path.join(dict_file_name);

        // Decompress a tarball
        let mut tar_gz = File::open(&source_dict_file_path)?;
        let mut buffer = Vec::new();
        tar_gz.read_to_end(&mut buffer)?;
        let cursor = Cursor::new(buffer);
        let gzdecoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(gzdecoder);
        archive.unpack(&build_dir)?;
    }

    // Lindera IPADIC directory
    let output_dir = Path::new(&build_dir).join("lindera-ko-dic");

    // Build a dictionary
    let builder = KoDicBuilder::new();
    builder.build_dictionary(&input_dir, &output_dir)?;

    Ok(())
}

#[cfg(not(feature = "ko-dic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}
