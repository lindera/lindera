//! Build script of `lindera-dictionary`.
//!
//! With the `build_rs` feature, which the dictionary crates' build scripts
//! use, it hashes this crate's source and passes the digest to the crate as
//! `LINDERA_DICTIONARY_SOURCE_DIGEST`. `assets.rs` records the digest with
//! each dictionary it builds into `LINDERA_BUILD_DICTIONARY_CACHE_DIR`, so a
//! dictionary cached by other builder code is rebuilt rather than reused
//! (#1132). Without the feature it does nothing.

#[cfg(feature = "build_rs")]
#[path = "src/source_digest.rs"]
mod source_digest;

/// The files and directories, relative to the crate root, whose contents
/// make up the source digest.
#[cfg(feature = "build_rs")]
const DIGEST_INPUTS: [&str; 2] = ["src", "Cargo.toml"];

/// Prints the `cargo:` directives of this build script.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "build_rs")]
    emit_source_digest();
}

/// Hashes [`DIGEST_INPUTS`] and passes the digest to the crate.
///
/// The digest is computed in the same build step that compiles the crate, so
/// it can lag behind the compiled code (a file saved during the build) but
/// never run ahead of it; cargo reruns this script on the next build, and the
/// cached dictionaries are rebuilt then. When the source cannot be read, no
/// digest is passed and the cache is not used.
#[cfg(feature = "build_rs")]
fn emit_source_digest() {
    for input in DIGEST_INPUTS {
        println!("cargo:rerun-if-changed={input}");
    }
    let root = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    match source_digest::collect_files(&root, &DIGEST_INPUTS) {
        Ok(files) => println!(
            "cargo:rustc-env=LINDERA_DICTIONARY_SOURCE_DIGEST={}",
            source_digest::digest_files(files)
        ),
        Err(err) => println!(
            "cargo:warning=cannot hash the lindera-dictionary source ({err}); \
             LINDERA_BUILD_DICTIONARY_CACHE_DIR will not reuse cached dictionaries"
        ),
    }
}
