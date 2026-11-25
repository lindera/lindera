//! Embeds the model of a dictionary, for the features that ask for one
//! (`embed-model-ipadic`); does nothing in any other build.
//!
//! The model is not built here: its training is in this crate and needs a
//! corpus. A published crate takes it from the archive that its release put
//! on the page of its version, pinned by `prebuilt.json`; a build from the
//! repository takes the file that `LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE`
//! names, which `scripts/dependency/build_model.sh` writes.

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "embed-model-ipadic")]
    lindera_dictionary::assets::build_embedded_file(
        true,
        lindera_dictionary::assets::PublishedFile {
            key: "lindera-dependency-ipadic",
            file: "dependency.model",
            // The version of the model file format this crate reads; a test
            // of the `embedded` module keeps it equal to the one of `model`.
            format_version: 1,
            file_env: "LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE",
            output: "dependency-ipadic.model",
            how_to_make: "build the model with scripts/dependency/build_model.sh of the \
                          Lindera repository",
        },
    )?;
    #[cfg(not(feature = "embed-model-ipadic"))]
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
