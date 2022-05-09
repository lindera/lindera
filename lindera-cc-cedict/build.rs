use std::error::Error;

#[cfg(feature = "cc-cedict")]
fn main() -> Result<(), Box<dyn Error>> {
    use std::env;
    use std::fs::{self, create_dir, File};
    use std::io::{self, Write};
    use std::path::Path;

    use encoding::all::UTF_8;
    use encoding::{EncoderTrap, Encoding};
    use zip::ZipArchive;

    use lindera_cc_cedict_builder::cc_cedict_builder::CcCedictBuilder;
    use lindera_core::dictionary_builder::DictionaryBuilder;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Directory path for build package
    let build_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    // UniDic MeCab directory
    let input_dir = Path::new(&build_dir).join("CC-CEDICT-MeCab-master");

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
        // Resources directory
        let resources_dir_path = Path::new("resources");

        // Dictionary file name
        let dict_file_name = "cc-cedict-mecab-master-20220509.zip";

        // Source dictionary file path
        let source_dict_file_path = resources_dir_path.join(dict_file_name);

        // Unzip
        let zip_file = File::open(&source_dict_file_path)?;
        let mut archive = ZipArchive::new(zip_file)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let output_file_path = match file.enclosed_name() {
                Some(path) => Path::new(&build_dir).join(path),
                None => continue,
            };
            {
                let comment = file.comment();
                if !comment.is_empty() {
                    println!("File {} comment: {}", i, comment);
                }
            }
            if (*file.name()).ends_with('/') {
                println!("File {} extracted to \"{}\"", i, output_file_path.display());
                fs::create_dir_all(&output_file_path)?;
            } else {
                println!(
                    "File {} extracted to \"{}\" ({} bytes)",
                    i,
                    output_file_path.display(),
                    file.size()
                );
                if let Some(p) = output_file_path.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = fs::File::create(&output_file_path)?;
                io::copy(&mut file, &mut outfile)?;
            }
            // Get and Set permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&output_file_path, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    // Lindera CC-CEDICT directory
    let output_dir = Path::new(&build_dir).join("lindera-cc-cedict");

    // Build a dictionary
    let builder = CcCedictBuilder::new();
    builder.build_dictionary(&input_dir, &output_dir)?;

    Ok(())
}

#[cfg(not(feature = "cc-cedict"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}
