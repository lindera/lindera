//! Dictionary management for morphological analysis.
//!
//! This module provides functionality for building, loading, and managing dictionaries
//! used in morphological analysis.

use std::path::Path;

use lindera::dictionary::{
    Dictionary, DictionaryBuilder, Metadata, UserDictionary,
    load_dictionary as lindera_load_dictionary,
    load_user_dictionary as lindera_load_user_dictionary,
};

use crate::error::to_napi_error;
use crate::metadata::JsMetadata;

/// A morphological analysis dictionary.
///
/// Contains the data structures needed for tokenization and morphological analysis.
#[napi(js_name = "Dictionary")]
pub struct JsDictionary {
    pub(crate) inner: Dictionary,
}

#[napi]
impl JsDictionary {
    /// Returns the name of the dictionary metadata.
    #[napi]
    pub fn metadata_name(&self) -> String {
        self.inner.metadata.name.clone()
    }

    /// Returns the character encoding of the dictionary.
    #[napi]
    pub fn metadata_encoding(&self) -> String {
        self.inner.metadata.encoding.clone()
    }

    /// Returns the full metadata object of the dictionary.
    #[napi]
    pub fn metadata(&self) -> JsMetadata {
        JsMetadata::from(self.inner.metadata.clone())
    }
}

/// A user-defined dictionary for custom words.
///
/// User dictionaries allow you to add custom words and their morphological features
/// that are not present in the main dictionary.
#[napi(js_name = "UserDictionary")]
pub struct JsUserDictionary {
    pub(crate) inner: UserDictionary,
}

/// Builds a dictionary from source files.
///
/// # Arguments
///
/// * `input_dir` - Directory containing dictionary source files.
/// * `output_dir` - Directory where the built dictionary will be saved.
/// * `metadata` - Metadata configuration for the dictionary.
#[napi]
pub fn build_dictionary(
    input_dir: String,
    output_dir: String,
    metadata: &JsMetadata,
) -> napi::Result<()> {
    let input_path = Path::new(&input_dir);
    let output_path = Path::new(&output_dir);

    if !input_path.exists() {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("Input directory does not exist: {input_dir}"),
        ));
    }

    let meta: Metadata = JsMetadata::to_lindera_metadata(metadata);
    let builder = DictionaryBuilder::new(meta);

    builder
        .build_dictionary(input_path, output_path)
        .map_err(|e| to_napi_error(format!("Failed to build dictionary: {e}")))?;

    Ok(())
}

/// Builds a user dictionary from a CSV file.
///
/// # Arguments
///
/// * `kind` - Dictionary kind (reserved for future use).
/// * `input_file` - Path to the CSV file containing user dictionary entries.
/// * `output_dir` - Directory where the built user dictionary will be saved.
/// * `metadata` - Optional metadata configuration. If omitted, default values are used.
#[napi]
pub fn build_user_dictionary(
    _kind: String,
    input_file: String,
    output_dir: String,
    metadata: Option<&JsMetadata>,
) -> napi::Result<()> {
    let input_path = Path::new(&input_file);
    let output_path = Path::new(&output_dir);

    if !input_path.exists() {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("Input file does not exist: {input_file}"),
        ));
    }

    let meta = match metadata {
        Some(m) => JsMetadata::to_lindera_metadata(m),
        None => Metadata::default(),
    };

    let builder = DictionaryBuilder::new(meta);

    builder
        .build_user_dictionary(input_path, output_path)
        .map_err(|e| to_napi_error(format!("Failed to build user dictionary: {e}")))?;

    Ok(())
}

/// Loads a dictionary from the specified URI.
///
/// # Arguments
///
/// * `uri` - URI to the dictionary. Can be a file path or embedded dictionary URI
///   (e.g. "embedded://ipadic").
///
/// # Returns
///
/// A loaded Dictionary object.
#[napi]
pub fn load_dictionary(uri: String) -> napi::Result<JsDictionary> {
    lindera_load_dictionary(&uri)
        .map_err(|e| to_napi_error(format!("Failed to load dictionary from '{uri}': {e}")))
        .map(|inner| JsDictionary { inner })
}

/// Loads a user dictionary from the specified URI.
///
/// # Arguments
///
/// * `uri` - URI to the user dictionary directory.
/// * `metadata` - Metadata configuration for the user dictionary.
///
/// # Returns
///
/// A loaded UserDictionary object.
#[napi]
pub fn load_user_dictionary(uri: String, metadata: &JsMetadata) -> napi::Result<JsUserDictionary> {
    let meta: Metadata = JsMetadata::to_lindera_metadata(metadata);
    lindera_load_user_dictionary(&uri, &meta)
        .map_err(|e| to_napi_error(format!("Failed to load user dictionary from '{uri}': {e}")))
        .map(|inner| JsUserDictionary { inner })
}
