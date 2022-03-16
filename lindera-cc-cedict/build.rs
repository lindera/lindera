use std::error::Error;

#[cfg(feature = "cc-cedict")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use std::env;
    use std::fs;
    use std::fs::File;
    use std::fs::{create_dir, rename};
    use std::io;
    use std::io::Write;
    use std::path::Path;

    use encoding::all::UTF_8;
    use encoding::{EncoderTrap, Encoding};
    use zip::ZipArchive;

    use lindera_cc_cedict_builder::cc_cedict_builder::CedictBuilder;
    use lindera_core::dictionary_builder::DictionaryBuilder;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Directory path for build package
    let build_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    // Dictionary file name
    let file_name = "CC-CEDICT-MeCab-master.zip";

    // UniDic MeCab directory
    let input_dir = Path::new(&build_dir).join("CC-CEDICT-MeCab-master");

    // Lindera IPADIC directory
    let output_dir = Path::new(&build_dir).join("lindera-cc-cedict");

    if std::env::var("DOCS_RS").is_ok() {
        // Use dummy data in docs.rs.
        create_dir(&input_dir)?;

        let mut dummy_char_def = File::create(input_dir.join("char.def"))?;
        dummy_char_def.write_all(b"DEFAULT 0 1 0\n")?;

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

        File::create(input_dir.join("unk.def"))?;
        let mut dummy_matrix_def = File::create(input_dir.join("matrix.def"))?;
        dummy_matrix_def.write_all(b"0 1 0\n")?;
    } else {
        // Source file path for build package
        let source_path_for_build = Path::new(&build_dir).join(&file_name);

        // Download source file to build directory
        if !source_path_for_build.exists() {
            // copy(&source_path, &source_path_for_build)?;
            let tmp_path = Path::new(&build_dir).join(file_name.to_owned() + ".download");

            // Download a tarball
            let download_url =
                "https://github.com/ueda-keisuke/CC-CEDICT-MeCab/archive/refs/heads/master.zip";
            let mut resp = reqwest::get(download_url).await?;

            // Save a ttarball
            let mut dest = File::create(&tmp_path)?;
            while let Some(chunk) = resp.chunk().await? {
                dest.write_all(&chunk)?;
            }
            rename(tmp_path, &source_path_for_build).expect("Failed to rename temporary file");
        }

        // Unzip
        let zip_file = File::open(&source_path_for_build)?;
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

    // Build a dictionary
    let builder = CedictBuilder::new();
    builder.build_dictionary(&input_dir, &output_dir)?;

    Ok(())
}

#[cfg(not(feature = "cc-cedict"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}
