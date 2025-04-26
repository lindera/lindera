use std::error::Error;
use std::path::Path;

use log::debug;
use rand::rng;

use crate::dictionary_builder::DictionaryBuilder;

const MAX_ROUND: usize = 3;

pub struct FetchParams {
    /// Dictionary file name
    pub file_name: &'static str,

    /// MeCab directory
    pub input_dir: &'static str,

    /// Lindera directory
    pub output_dir: &'static str,

    /// Dummy input for docs.rs
    pub dummy_input: &'static str,

    /// URLs from which to fetch the asset, with their MD5 checksums
    pub download_urls: &'static [(&'static str, &'static str)],
}

#[cfg(not(target_os = "windows"))]
fn empty_directory(dir: &Path) -> Result<(), Box<dyn Error>> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), Box<dyn Error>> {
    if !dst.exists() {
        std::fs::create_dir(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &dst_path)?;
        } else {
            std::fs::copy(&entry_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Fetch the necessary assets and then build the dictionary using `builder`
pub async fn fetch(
    params: FetchParams,
    builder: impl DictionaryBuilder,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    use std::fs::{create_dir, rename, File};
    use std::io::{self, Cursor, Read, Write};
    use std::path::{Path, PathBuf};

    use encoding::all::UTF_8;
    use encoding::{EncoderTrap, Encoding};
    use flate2::read::GzDecoder;
    use md5::Context;
    use rand::seq::SliceRandom;
    use reqwest::Client;
    use tar::Archive;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Directory path for build package
    // if the `LINDERA_CACHE` variable is defined, behaves like a cache, where data is invalidated only:
    // - on new lindera-assets version
    // - if the LINDERA_CACHE dir changed
    // otherwise, keeps behavior of always redownloading and rebuilding
    let (build_dir, is_cache) = if let Some(lindera_cache_dir) = env::var_os("LINDERA_CACHE") {
        (
            PathBuf::from(lindera_cache_dir).join(env::var_os("CARGO_PKG_VERSION").unwrap()),
            true,
        )
    } else {
        (
            PathBuf::from(env::var_os("OUT_DIR").unwrap()), /* ex) target/debug/build/<pkg>/out */
            false,
        )
    };

    // environment variable passed to dependents, that will actually be used to include the dictionary in the library
    println!("cargo::rustc-env=LINDERA_WORKDIR={}", build_dir.display());

    std::fs::create_dir_all(&build_dir)?;

    let input_dir = build_dir.join(params.input_dir);

    let output_dir = build_dir.join(params.output_dir);

    // Fast path where the data is already in cache
    if is_cache && output_dir.is_dir() {
        return Ok(());
    }

    if std::env::var("DOCS_RS").is_ok() {
        // Create directory for dummy input directory for build docs
        create_dir(&input_dir)?;

        // Create dummy char.def
        let mut dummy_char_def = File::create(input_dir.join("char.def"))?;
        dummy_char_def.write_all(b"DEFAULT 0 1 0\n")?;

        // Create dummy CSV file
        let mut dummy_dict_csv = File::create(input_dir.join("dummy_dict.csv"))?;
        dummy_dict_csv.write_all(
            &UTF_8
                .encode(params.dummy_input, EncoderTrap::Ignore)
                .unwrap(),
        )?;

        // Create dummy unk.def
        File::create(input_dir.join("unk.def"))?;
        let mut dummy_matrix_def = File::create(input_dir.join("matrix.def"))?;
        dummy_matrix_def.write_all(b"0 1 0\n")?;
    } else {
        // Source file path for build package
        let source_path_for_build = &build_dir.join(params.file_name);

        // Download source file to build directory
        let tmp_path = Path::new(&build_dir).join(params.file_name.to_owned() + ".download");

        // Download a tarball
        let client = Client::builder()
            .user_agent(format!("Lindera/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        let mut last_error = None;
        let mut round = 0;

        while round < MAX_ROUND {
            let mut download_urls = params.download_urls.to_vec();
            download_urls.shuffle(&mut rng());

            for (file_url, md5_url) in &download_urls {
                debug!(
                    "Round {}: Downloading dictionary from {}",
                    round + 1,
                    file_url
                );

                let mut dest = File::create(&tmp_path)?;
                let resp = client.get(*file_url).send().await;

                if let Ok(resp) = resp {
                    if resp.status().is_success() {
                        let content = resp.bytes().await?;
                        io::copy(&mut content.as_ref(), &mut dest)?;
                        dest.flush()?;

                        debug!("Content-Length: {}", content.len());
                        debug!("Downloaded to {}", tmp_path.display());

                        // Download and verify MD5 checksum
                        debug!("Downloading md5 file from {}", md5_url);
                        let md5_resp = client.get(*md5_url).send().await?;

                        if !md5_resp.status().is_success() {
                            debug!("Failed to download md5: {}", md5_resp.status());
                            last_error =
                                Some(format!("MD5 download failed: {}", md5_resp.status()));
                            continue;
                        }

                        let md5_text = md5_resp.text().await?.trim().to_string();

                        // Calculate MD5 checksum of the downloaded file
                        let mut file = File::open(&tmp_path)?;
                        let mut buf = Vec::new();
                        file.read_to_end(&mut buf)?;

                        let mut context = Context::new();
                        context.consume(&buf);
                        let file_md5 = format!("{:x}", context.compute());

                        debug!("Downloaded file MD5: {}", file_md5);
                        debug!("Expected MD5 from .md5 file: {}", md5_text);

                        if file_md5 == md5_text {
                            debug!("Checksum matched!");
                            rename(&tmp_path, source_path_for_build)
                                .expect("Failed to rename temporary file");
                            last_error = None;
                            break;
                        } else {
                            debug!("Checksum mismatch, trying next...");
                            last_error = Some("Checksum mismatch".to_string());
                            continue;
                        }
                    } else {
                        debug!("Failed with status: {}", resp.status());
                        last_error = Some(format!("Download failed: {}", resp.status()));
                        continue;
                    }
                } else {
                    debug!("Download request failed");
                    last_error = Some("Request error".to_string());
                    continue;
                }
            }

            if last_error.is_none() {
                break; // Exit the loop if download was successful
            }

            round += 1;

            // Suffle the download URLs for the next round
            download_urls.shuffle(&mut rng());
        }

        if let Some(error) = last_error {
            return Err(error.into());
        }

        // Decompress a tar.gz file
        let tmp_extract_path =
            Path::new(&build_dir).join(format!("tmp-archive-{}", params.input_dir));
        let tmp_extracted_path = tmp_extract_path.join(params.input_dir);
        let _ = std::fs::remove_dir_all(&tmp_extract_path);
        std::fs::create_dir_all(&tmp_extract_path)?;

        let mut tar_gz = File::open(source_path_for_build)?;
        let mut buffer = Vec::new();
        tar_gz.read_to_end(&mut buffer)?;
        let cursor = Cursor::new(buffer);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);
        archive.unpack(&tmp_extract_path)?;

        #[cfg(target_os = "windows")]
        {
            // Recreate input_dir to avoid conflicts when copying the directory on Windows systems (which do not support overwriting directories).
            // Check if output_dir exists
            if input_dir.exists() {
                // Remove input_dir
                std::fs::remove_dir_all(&input_dir).expect("Failed to remove input directory");

                // Make input_dir
                std::fs::create_dir_all(&input_dir).expect("Failed to create input directory");
            }

            // Copy tmp_path to input_dir
            copy_dir_all(&tmp_extracted_path, &input_dir)
                .expect("Failed to copy files from temporary directory to input directory");

            // remove tmp_path
            std::fs::remove_dir_all(&tmp_extracted_path)
                .expect("Failed to remove temporary directory");
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Empty the input directory first to avoid conflicts when renaming the directory later on Linux and macOS systems (which do not support overwriting directories).
            empty_directory(&input_dir).expect("Failed to empty input directory");
            rename(tmp_extracted_path, &input_dir).expect("Failed to rename archive directory");
        }

        let _ = std::fs::remove_dir_all(&tmp_extract_path);
        let _ = std::fs::remove_file(source_path_for_build);
    }

    let tmp_path = build_dir.join(format!("tmp-output-{}", params.output_dir));
    let _ = std::fs::remove_dir_all(&tmp_path);

    builder.build_dictionary(&input_dir, &tmp_path)?;

    #[cfg(target_os = "windows")]
    {
        // Check if output_dir exists
        if output_dir.exists() {
            // Remove output_dir
            std::fs::remove_dir_all(&output_dir).expect("Failed to remove output directory");

            // Make output_dir
            std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");
        }

        // Copy tmp_path to output_dir
        copy_dir_all(&tmp_path, &output_dir).expect("Failed to copy output directory");

        // remove tmp_path
        std::fs::remove_dir_all(&tmp_path).expect("Failed to copy output directory");
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Empty the output directory
        empty_directory(&output_dir).expect("Failed to empty output directory");

        // Rename tmp_path to output_dir
        rename(tmp_path, &output_dir).expect("Failed to rename output directory");
    }

    let _ = std::fs::remove_dir_all(&input_dir);

    Ok(())
}
