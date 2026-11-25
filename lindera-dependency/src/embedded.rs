//! The models embedded in the build.
//!
//! A feature of the crate embeds the model that a release of Lindera
//! publishes for a dictionary: `embed-model-ipadic` the one for IPADIC. The
//! bytes are part of the program, and [`model`] reads them where they are.
//!
//! A model is named by its dictionary, and written as the embedded
//! dictionary is, `embedded://ipadic`: what [`dictionary_of`] reads, for the
//! command and the token filter, which take a path or this.
//!
//! ```
//! use lindera_dependency::embedded;
//!
//! assert_eq!(embedded::dictionary_of("embedded://ipadic"), Some("ipadic"));
//! assert_eq!(embedded::dictionary_of("models/ipadic.model"), None);
//!
//! match embedded::model("ipadic") {
//!     // A build with `embed-model-ipadic`.
//!     Ok(model) => assert_eq!(model.dictionary(), "ipadic"),
//!     // Any other build: the error names the feature.
//!     Err(error) => assert!(error.to_string().contains("embed-model-ipadic")),
//! }
//! ```
//!
//! Where the bytes come from at build time is in the model file page of the
//! documentation: the archive of a release for a build of the published
//! crate, and a file named by `LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE`
//! for a build from the repository.

use crate::error::{Error, Result};
use crate::model::Model;

/// What a model embedded in the build is written with, before the name of
/// its dictionary.
pub const SCHEME: &str = "embedded://";

/// The dictionaries a feature of this crate can embed a model for, each
/// with its feature.
const KNOWN: [(&str, &str); 1] = [("ipadic", "embed-model-ipadic")];

/// The model for IPADIC, as the build script put it in `OUT_DIR`.
#[cfg(feature = "embed-model-ipadic")]
static IPADIC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dependency-ipadic.model"));

/// The dictionary of an embedded model, from how it is written.
///
/// # Arguments
///
/// * `model` - What names a model: a path, or `embedded://<dictionary>`.
///
/// # Returns
///
/// The name of the dictionary for `embedded://<dictionary>`, and `None` for
/// anything else, which is then a path.
pub fn dictionary_of(model: &str) -> Option<&str> {
    model.strip_prefix(SCHEME)
}

/// The bytes of the model embedded for a dictionary.
///
/// # Arguments
///
/// * `dictionary` - The name of the dictionary, as `ipadic`.
///
/// # Returns
///
/// The bytes of the model file, or an [`Error::Embedded`]: for a dictionary
/// no feature embeds a model for, for a build without the feature (the
/// message names it), and for a documentation build, which embeds an empty
/// file.
pub fn bytes(dictionary: &str) -> Result<&'static [u8]> {
    let Some((_, feature)) = KNOWN.iter().find(|(name, _)| *name == dictionary) else {
        let known: Vec<&str> = KNOWN.iter().map(|(name, _)| *name).collect();
        return Err(Error::Embedded(format!(
            "no model can be embedded for the dictionary {dictionary}: there are models for {}",
            known.join(", ")
        )));
    };
    let embedded: Option<&'static [u8]> = match dictionary {
        #[cfg(feature = "embed-model-ipadic")]
        "ipadic" => Some(IPADIC),
        _ => None,
    };
    answer(dictionary, feature, embedded)
}

/// What [`bytes`] answers for what the build embedded.
///
/// # Arguments
///
/// * `dictionary` - The name of the dictionary.
/// * `feature` - The feature that embeds its model.
/// * `embedded` - The bytes the build embedded, or `None` for a build
///   without the feature.
///
/// # Returns
///
/// The bytes, or an [`Error::Embedded`] for none and for an empty file.
fn answer(
    dictionary: &str,
    feature: &str,
    embedded: Option<&'static [u8]>,
) -> Result<&'static [u8]> {
    match embedded {
        None => Err(Error::Embedded(format!(
            "the model for {dictionary} is not embedded in this build: it needs a build with the \
             feature `{feature}` of lindera-dependency, which lindera, its command and its \
             bindings forward as `embed-dependency-{dictionary}`"
        ))),
        Some([]) => Err(Error::Embedded(format!(
            "the model for {dictionary} is empty in this build: a documentation build embeds no \
             model"
        ))),
        Some(bytes) => Ok(bytes),
    }
}

/// The model embedded for a dictionary.
///
/// # Arguments
///
/// * `dictionary` - The name of the dictionary, as `ipadic`.
///
/// # Returns
///
/// The model, which reads the bytes of the program in place; or the error
/// of [`bytes`], or of [`Model::new`] for bytes that are not a model file of
/// this version.
pub fn model(dictionary: &str) -> Result<Model<&'static [u8]>> {
    Model::new(bytes(dictionary)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_is_named_as_an_embedded_dictionary_is() {
        assert_eq!(dictionary_of("embedded://ipadic"), Some("ipadic"));
        assert_eq!(dictionary_of("embedded://"), Some(""));
        assert_eq!(dictionary_of("ipadic"), None);
        assert_eq!(dictionary_of("/models/embedded://ipadic"), None);
        assert_eq!(dictionary_of("file:///models/ipadic.model"), None);
    }

    #[test]
    fn a_dictionary_without_a_model_names_the_ones_there_are() {
        let message = bytes("unidic").unwrap_err().to_string();
        assert!(message.contains("unidic"), "{message}");
        assert!(message.contains("models for ipadic"), "{message}");
        assert!(matches!(model("").unwrap_err(), Error::Embedded(_)));
    }

    #[cfg(not(feature = "embed-model-ipadic"))]
    #[test]
    fn a_build_without_the_feature_names_it() {
        let message = bytes("ipadic").unwrap_err().to_string();
        assert!(message.contains("not embedded in this build"), "{message}");
        assert!(message.contains("`embed-model-ipadic`"), "{message}");
        assert!(message.contains("`embed-dependency-ipadic`"), "{message}");
        assert!(matches!(model("ipadic").unwrap_err(), Error::Embedded(_)));
    }

    #[cfg(feature = "embed-model-ipadic")]
    #[test]
    fn the_embedded_model_is_for_its_dictionary() {
        let model = model("ipadic").expect("embedded model");
        assert_eq!(model.dictionary(), "ipadic");
        assert!(model.chunker().is_some());
        assert!(model.parser().is_some());
    }

    #[test]
    fn the_empty_file_of_a_documentation_build_is_an_error() {
        let message = answer("ipadic", "embed-model-ipadic", Some(&[]))
            .unwrap_err()
            .to_string();
        assert!(message.contains("is empty in this build"), "{message}");
        assert_eq!(
            answer("ipadic", "embed-model-ipadic", Some(b"bytes")).unwrap(),
            b"bytes"
        );
        let message = answer("ipadic", "embed-model-ipadic", None)
            .unwrap_err()
            .to_string();
        assert!(message.contains("`embed-model-ipadic`"), "{message}");
    }

    #[test]
    fn the_build_script_pins_the_format_version_this_crate_reads() {
        let script = include_str!("../build.rs");
        let line = format!("format_version: {},", crate::model::VERSION);
        assert!(script.contains(&line), "build.rs has no `{line}`");
    }
}
