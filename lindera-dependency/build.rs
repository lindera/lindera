//! Embeds the model of a dictionary, for the features that ask for one
//! (`embed-model-ipadic`, `embed-model-unidic`, `embed-model-unidic-cwj`,
//! `embed-model-unidic-csj`); does nothing in any other build.
//!
//! A model is not built here: its training is in this crate and needs a
//! corpus. A published crate takes it from the archive that its release put
//! on the page of its version, pinned by the `prebuilt-<dictionary>.json` of
//! the model; a build from the repository takes the file that the variable
//! of the model names, `LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE` for
//! IPADIC, which `scripts/dependency/build_model.sh` writes.

use std::error::Error;

/// The models a feature embeds: whether the feature is on, the key of the
/// archive, the pin, the variable, and the name of the file in `OUT_DIR`.
/// `src/embedded.rs` includes the files, and a test of it keeps the two
/// lists the same.
const MODELS: [(bool, &str, &str, &str, &str); 4] = [
    (
        cfg!(feature = "embed-model-ipadic"),
        "lindera-dependency-ipadic",
        "prebuilt-ipadic.json",
        "LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE",
        "dependency-ipadic.model",
    ),
    (
        cfg!(feature = "embed-model-unidic"),
        "lindera-dependency-unidic",
        "prebuilt-unidic.json",
        "LINDERA_BUILD_DEPENDENCY_MODEL_UNIDIC_FILE",
        "dependency-unidic.model",
    ),
    (
        cfg!(feature = "embed-model-unidic-cwj"),
        "lindera-dependency-unidic-cwj",
        "prebuilt-unidic-cwj.json",
        "LINDERA_BUILD_DEPENDENCY_MODEL_UNIDIC_CWJ_FILE",
        "dependency-unidic-cwj.model",
    ),
    (
        cfg!(feature = "embed-model-unidic-csj"),
        "lindera-dependency-unidic-csj",
        "prebuilt-unidic-csj.json",
        "LINDERA_BUILD_DEPENDENCY_MODEL_UNIDIC_CSJ_FILE",
        "dependency-unidic-csj.model",
    ),
];

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(any(
        feature = "embed-model-ipadic",
        feature = "embed-model-unidic",
        feature = "embed-model-unidic-cwj",
        feature = "embed-model-unidic-csj"
    ))]
    for (enabled, key, pin, file_env, output) in MODELS {
        if !enabled {
            continue;
        }
        lindera_dictionary::assets::build_embedded_file(
            true,
            lindera_dictionary::assets::PublishedFile {
                key,
                pin,
                file: "dependency.model",
                // The version of the model file format this crate reads; a
                // test of the `embedded` module keeps it equal to the one
                // of `model`.
                format_version: 3,
                file_env,
                output,
                how_to_make: "build the model with scripts/dependency/build_model.sh of the \
                              Lindera repository",
            },
        )?;
    }
    // Read by the test of `src/embedded.rs` only, in a build without a
    // feature that embeds a model.
    let _ = MODELS;
    Ok(())
}
