use std::error::Error;
use std::fs::{self, File, rename};
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};

use encoding::all::UTF_8;
use encoding::{EncoderTrap, Encoding};
use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use md5::Context;
use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use reqwest::Client;
use tar::Archive;
use tokio::time::{Duration, sleep};

use crate::LinderaResult;
use crate::builder::DictionaryBuilder;
use crate::error::LinderaErrorKind;

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

    /// URLs from which to fetch the asset
    pub download_urls: &'static [&'static str],

    /// MD5 hash of the file
    pub md5_hash: &'static str,
}

#[cfg(target_os = "windows")]
fn copy_dir_all(src: &Path, dst: &Path) -> LinderaResult<()> {
    if !dst.is_dir() {
        fs::create_dir_all(dst).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to create directory: {dst:?}"))
        })?;
    }

    for entry in fs::read_dir(src).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to read directory: {src:?}"))
    })? {
        let entry = entry.map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to get directory entry in: {src:?}"))
        })?;
        let entry_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &dst_path)?;
        } else {
            fs::copy(&entry_path, &dst_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to copy file: {entry_path:?} to {dst_path:?}"
                    ))
            })?;
        }
    }
    Ok(())
}

fn empty_directory(dir: &Path) -> LinderaResult<()> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to remove directory: {dir:?}"))
        })?;
    }

    fs::create_dir_all(dir).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to create directory: {dir:?}"))
    })?;

    Ok(())
}

fn rename_directory(dir: &Path, new_dir: &Path) -> LinderaResult<()> {
    // Ensure parent directory of new_dir exists
    if let Some(parent) = new_dir.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to create parent directory: {parent:?}"))
        })?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        rename(dir, new_dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to rename directory: {dir:?} to {new_dir:?}"
                ))
        })?;
    }

    #[cfg(target_os = "windows")]
    {
        copy_dir_all(dir, new_dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!("{err}"))
                .add_context(format!("Failed to copy directory: {dir:?} to {new_dir:?}"))
        })?;

        fs::remove_dir_all(dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to remove source directory after copy: {dir:?}"
                ))
        })?;
    }

    Ok(())
}

async fn download_with_retry(
    client: &Client,
    download_urls: Vec<&str>,
    max_rounds: usize,
    expected_md5: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if download_urls.is_empty() {
        return Err("No download URLs provided".into());
    }

    for round in 0..max_rounds {
        let mut urls = download_urls.clone();

        let mut rng = SmallRng::seed_from_u64(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
        );
        urls.shuffle(&mut rng);

        debug!(
            "Round {}/{}: Trying {} URLs",
            round + 1,
            max_rounds,
            urls.len()
        );

        for url in urls {
            debug!("Attempting to download from {url}");
            match client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    debug!("HTTP download successful from {url}");

                    match resp.bytes().await {
                        Ok(content) => {
                            // Calculate MD5 hash
                            let mut context = Context::new();
                            context.consume(&content);
                            let actual_md5 = format!("{:x}", context.finalize());

                            debug!("Expected MD5: {expected_md5}");
                            debug!("Actual   MD5: {actual_md5}");

                            if actual_md5 == expected_md5 {
                                debug!("MD5 check passed from {url}");
                                return Ok(content.to_vec());
                            } else {
                                warn!(
                                    "MD5 mismatch from {url}, Expected {expected_md5}, got {actual_md5}"
                                );
                                // continue to next url
                            }
                        }
                        Err(e) => {
                            warn!("Failed to download content from {url}: {e}");
                            // continue to next url
                        }
                    }
                }
                Ok(resp) => {
                    warn!("HTTP download failed from {}: HTTP {}", url, resp.status());
                    // continue to next url
                }
                Err(e) => {
                    warn!("Request error from {url}: {e}");
                    // continue to next url
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
    }

    error!("All {max_rounds} attempts failed");
    Err("Failed to download a valid file from all sources".into())
}

/// Fetch the necessary assets and then build the dictionary using `builder`
pub async fn fetch(params: FetchParams, builder: DictionaryBuilder) -> LinderaResult<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=LINDERA_DICTS");
    println!("cargo:rerun-if-env-changed=LINDERA_CACHE");
    println!("cargo:rerun-if-env-changed=DOCS_RS");

    // Directory path for build package
    // if the `LINDERA_DICTS` variable is defined, behaves like a cache, where data is invalidated only:
    // - on new lindera-assets version
    // - if the LINDERA_DICTS dir changed
    // otherwise, keeps behavior of always redownloading and rebuilding
    let (build_dir, is_cache) = if let Some(path) =
        std::env::var_os("LINDERA_DICTS").or_else(|| {
            std::env::var_os("LINDERA_CACHE").map(|p| {
                println!(
                    "cargo:warning=LINDERA_CACHE is deprecated. Please use LINDERA_DICTS instead."
                );
                p
            })
        }) {
        let mut cache_dir = PathBuf::from(path);
        if !cache_dir.is_absolute() {
            if let Ok(current_dir) = std::env::current_dir() {
                // If current_dir is a crate directory in a workspace, try to find the workspace root
                let mut root_dir = current_dir.clone();
                if let Some(parent) = current_dir.parent() {
                    if parent.join("Cargo.toml").exists() {
                        root_dir = parent.to_path_buf();
                    }
                }
                cache_dir = root_dir.join(cache_dir);
            }
        }

        (
            cache_dir.join(std::env::var_os("CARGO_PKG_VERSION").unwrap()),
            true,
        )
    } else {
        (
            PathBuf::from(std::env::var_os("OUT_DIR").unwrap()), /* ex) target/debug/build/<pkg>/out */
            false,
        )
    };

    // environment variable passed to dependents, that will actually be used to include the dictionary in the library
    println!("cargo::rustc-env=LINDERA_WORKDIR={}", build_dir.display());

    fs::create_dir_all(&build_dir).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to create build directory: {build_dir:?}"))
    })?;

    let input_dir = build_dir.join(params.input_dir);

    let output_dir = build_dir.join(params.output_dir);

    // Fast path where the data is already in cache
    if is_cache && output_dir.is_dir() {
        return Ok(());
    }

    if std::env::var("DOCS_RS").is_ok() {
        // Create directory for dummy input directory for build docs
        fs::create_dir(&input_dir).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to create dummy input directory: {input_dir:?}"
                ))
        })?;

        // Create dummy char.def
        let mut dummy_char_def = File::create(input_dir.join("char.def")).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to create dummy char.def: {:?}",
                    input_dir.join("char.def")
                ))
        })?;
        dummy_char_def
            .write_all(b"DEFAULT 0 1 0\n")
            .map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context("Failed to write to dummy char.def")
            })?;

        // Create dummy CSV file
        let mut dummy_dict_csv = File::create(input_dir.join("dummy_dict.csv")).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to create dummy CSV file: {:?}",
                    input_dir.join("dummy_dict.csv")
                ))
        })?;
        dummy_dict_csv
            .write_all(
                &UTF_8
                    .encode(params.dummy_input, EncoderTrap::Ignore)
                    .unwrap(),
            )
            .map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context("Failed to write to dummy CSV file")
            })?;

        // Create dummy unk.def
        File::create(input_dir.join("unk.def")).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to create dummy unk.def: {:?}",
                    input_dir.join("unk.def")
                ))
        })?;
        let mut dummy_matrix_def = File::create(input_dir.join("matrix.def")).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to create dummy matrix.def: {:?}",
                    input_dir.join("matrix.def")
                ))
        })?;
        dummy_matrix_def.write_all(b"0 1 0\n").map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to write to dummy matrix.def")
        })?;
    } else {
        // Source file path for build package
        let source_path_for_build = &build_dir.join(params.file_name);

        // Check if source file already exists and is valid
        let need_download = if source_path_for_build.exists() {
            debug!(
                "Found existing source file: {}",
                source_path_for_build.display()
            );

            // Verify MD5 hash
            let mut file = File::open(source_path_for_build).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to open source file for MD5 check: {source_path_for_build:?}"
                    ))
            })?;
            let mut context = Context::new();
            let mut buffer = [0; 8192];
            loop {
                let count = file.read(&mut buffer).map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context(format!(
                            "Failed to read source file for MD5 check: {source_path_for_build:?}"
                        ))
                })?;
                if count == 0 {
                    break;
                }
                context.consume(&buffer[..count]);
            }
            let actual_md5 = format!("{:x}", context.finalize());

            if actual_md5 == params.md5_hash {
                debug!("MD5 check passed for cached file. Skipping download.");
                false
            } else {
                warn!(
                    "MD5 mismatch for cached file. Expected: {}, Actual: {}",
                    params.md5_hash, actual_md5
                );
                // Remove invalid file
                fs::remove_file(source_path_for_build).map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context(format!(
                            "Failed to remove invalid source file: {source_path_for_build:?}"
                        ))
                })?;
                true
            }
        } else {
            debug!("Source file not found. Will download.");
            true
        };

        if need_download {
            // Download source file to build directory
            let tmp_download_path =
                Path::new(&build_dir).join(params.file_name.to_owned() + ".download");

            // Download a tarball
            let client = Client::builder()
                .user_agent(format!("Lindera/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context("Failed to build HTTP client")
                })?;

            debug!("Downloading {:?}", params.download_urls);
            let mut dest = File::create(tmp_download_path.as_path()).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create temporary download file: {tmp_download_path:?}"
                    ))
            })?;
            let content = download_with_retry(
                &client,
                params.download_urls.to_vec(),
                MAX_ROUND,
                params.md5_hash,
            )
            .await
            .map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!("{err}"))
                    .add_context("Failed to download dictionary assets")
            })?;

            io::copy(&mut Cursor::new(content.as_slice()), &mut dest).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to copy downloaded content to file: {tmp_download_path:?}"
                    ))
            })?;
            dest.flush().map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to flush download file: {tmp_download_path:?}"
                    ))
            })?;
            drop(dest);

            debug!("Content-Length: {}", content.len());
            debug!("Downloaded to {}", tmp_download_path.display());
            rename(tmp_download_path.clone(), source_path_for_build).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to rename temporary download file: {tmp_download_path:?} to {source_path_for_build:?}"
                    ))
            })?;

            info!("Source file cached at: {}", source_path_for_build.display());
        }

        // Decompress a tar.gz file
        let tmp_extract_path =
            Path::new(&build_dir).join(format!("tmp-archive-{}", params.input_dir));
        let tmp_extracted_path = tmp_extract_path.join(params.input_dir);
        let _ = fs::remove_dir_all(&tmp_extract_path);
        fs::create_dir_all(&tmp_extract_path).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to create temporary extraction directory: {tmp_extract_path:?}"
                ))
        })?;

        let mut tar_gz = File::open(source_path_for_build).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to open source file: {source_path_for_build:?}"
                ))
        })?;
        let mut buffer = Vec::new();
        tar_gz.read_to_end(&mut buffer).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to read source file: {source_path_for_build:?}"
                ))
        })?;
        let cursor = Cursor::new(buffer);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);
        archive.unpack(&tmp_extract_path).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to unpack archive: {source_path_for_build:?} to {tmp_extract_path:?}"
                ))
        })?;

        // Empty the input directory first to avoid conflicts when renaming the directory later on Linux and macOS systems (which do not support overwriting directories).
        empty_directory(&input_dir)?;

        rename_directory(&tmp_extracted_path, &input_dir)?;

        let _ = fs::remove_dir_all(&tmp_extract_path);
    }

    let tmp_output_path = build_dir.join(format!("tmp-output-{}", params.output_dir));
    let _ = fs::remove_dir_all(&tmp_output_path);

    builder
        .build_dictionary(&input_dir, &tmp_output_path)
        .map_err(|err| {
            LinderaErrorKind::Build
                .with_error(anyhow::anyhow!("{err}"))
                .add_context("Failed to build dictionary")
        })?;

    // Empty the output directory
    empty_directory(&output_dir)?;

    // Rename tmp_output_path to output_dir
    rename_directory(&tmp_output_path, &output_dir)?;

    let _ = fs::remove_dir_all(input_dir);

    Ok(())
}
