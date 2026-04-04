//! Dictionary management for morphological analysis.
//!
//! This module provides functionality for building, loading, and managing dictionaries
//! used in morphological analysis.

use std::path::Path;

use magnus::prelude::*;
use magnus::{Error, Ruby, function, method};

use lindera::dictionary::{
    Dictionary, DictionaryBuilder, Metadata, UserDictionary,
    load_dictionary as lindera_load_dictionary,
    load_user_dictionary as lindera_load_user_dictionary,
};

use crate::error::to_magnus_error;
use crate::metadata::RbMetadata;

/// A morphological analysis dictionary.
///
/// Contains the data structures needed for tokenization and morphological analysis.
#[magnus::wrap(class = "Lindera::Dictionary", free_immediately, size)]
#[derive(Clone)]
pub struct RbDictionary {
    /// Inner Lindera dictionary.
    pub inner: Dictionary,
}

impl RbDictionary {
    /// Returns the name of the dictionary metadata.
    ///
    /// # Returns
    ///
    /// The dictionary metadata name.
    fn metadata_name(&self) -> String {
        self.inner.metadata.name.clone()
    }

    /// Returns the character encoding of the dictionary.
    ///
    /// # Returns
    ///
    /// The dictionary encoding string.
    fn metadata_encoding(&self) -> String {
        self.inner.metadata.encoding.clone()
    }

    /// Returns the full metadata object.
    ///
    /// # Returns
    ///
    /// The dictionary metadata.
    fn metadata(&self) -> RbMetadata {
        RbMetadata::from(self.inner.metadata.clone())
    }

    /// Returns the string representation.
    fn to_s(&self) -> String {
        "Dictionary".to_string()
    }

    /// Returns the inspect representation.
    fn inspect(&self) -> String {
        format!(
            "#<Lindera::Dictionary: name='{}'>",
            self.inner.metadata.name
        )
    }
}

/// A user-defined dictionary for custom words.
///
/// User dictionaries allow you to add custom words and their morphological features
/// that are not present in the main dictionary.
#[magnus::wrap(class = "Lindera::UserDictionary", free_immediately, size)]
#[derive(Clone)]
pub struct RbUserDictionary {
    /// Inner Lindera user dictionary.
    pub inner: UserDictionary,
}

impl RbUserDictionary {
    /// Returns the string representation.
    fn to_s(&self) -> String {
        "UserDictionary".to_string()
    }

    /// Returns the inspect representation.
    fn inspect(&self) -> String {
        "UserDictionary()".to_string()
    }
}

/// Loads a dictionary from the specified URI.
///
/// # Arguments
///
/// * `uri` - URI to the dictionary. Can be a file path or embedded dictionary name.
///
/// # Returns
///
/// A loaded `RbDictionary` object.
fn load_dictionary(uri: String) -> Result<RbDictionary, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not initialized");
    lindera_load_dictionary(&uri)
        .map_err(|e| {
            to_magnus_error(
                &ruby,
                format!("Failed to load dictionary from '{uri}': {e}"),
            )
        })
        .map(|d| RbDictionary { inner: d })
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
/// A loaded `RbUserDictionary` object.
fn load_user_dictionary(uri: String, metadata: &RbMetadata) -> Result<RbUserDictionary, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not initialized");
    let meta: Metadata = metadata.clone().into();
    lindera_load_user_dictionary(&uri, &meta)
        .map_err(|e| {
            to_magnus_error(
                &ruby,
                format!("Failed to load user dictionary from '{uri}': {e}"),
            )
        })
        .map(|d| RbUserDictionary { inner: d })
}

/// Builds a dictionary from source files.
///
/// # Arguments
///
/// * `input_dir` - Directory containing dictionary source files.
/// * `output_dir` - Directory where the built dictionary will be saved.
/// * `metadata` - Metadata configuration for the dictionary.
fn build_dictionary(
    input_dir: String,
    output_dir: String,
    metadata: &RbMetadata,
) -> Result<(), Error> {
    let ruby = Ruby::get().expect("Ruby runtime not initialized");
    let input_path = Path::new(&input_dir);
    let output_path = Path::new(&output_dir);

    if !input_path.exists() {
        return Err(Error::new(
            ruby.exception_arg_error(),
            format!("Input directory does not exist: {input_dir}"),
        ));
    }

    let builder = DictionaryBuilder::new(metadata.clone().into());
    builder
        .build_dictionary(input_path, output_path)
        .map_err(|e| to_magnus_error(&ruby, format!("Failed to build dictionary: {e}")))?;

    Ok(())
}

/// Builds a user dictionary from a CSV file.
///
/// # Arguments
///
/// * `_kind` - Dictionary kind (reserved for future use).
/// * `input_file` - Path to the CSV file.
/// * `output_dir` - Directory where the built user dictionary will be saved.
/// * `metadata` - Optional metadata configuration.
fn build_user_dictionary(
    _kind: String,
    input_file: String,
    output_dir: String,
    metadata: Option<&RbMetadata>,
) -> Result<(), Error> {
    let ruby = Ruby::get().expect("Ruby runtime not initialized");
    let input_path = Path::new(&input_file);
    let output_path = Path::new(&output_dir);

    if !input_path.exists() {
        return Err(Error::new(
            ruby.exception_arg_error(),
            format!("Input file does not exist: {input_file}"),
        ));
    }

    let meta = match metadata {
        Some(m) => m.clone().into(),
        None => Metadata::default(),
    };

    let builder = DictionaryBuilder::new(meta);
    builder
        .build_user_dictionary(input_path, output_path)
        .map_err(|e| to_magnus_error(&ruby, format!("Failed to build user dictionary: {e}")))?;

    Ok(())
}

/// Defines Dictionary, UserDictionary classes and module functions in the given Ruby module.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `module` - Parent Ruby module.
///
/// # Returns
///
/// `Ok(())` on success, or a Magnus `Error` on failure.
pub fn define(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    let dict_class = module.define_class("Dictionary", ruby.class_object())?;
    dict_class.define_method("metadata_name", method!(RbDictionary::metadata_name, 0))?;
    dict_class.define_method(
        "metadata_encoding",
        method!(RbDictionary::metadata_encoding, 0),
    )?;
    dict_class.define_method("metadata", method!(RbDictionary::metadata, 0))?;
    dict_class.define_method("to_s", method!(RbDictionary::to_s, 0))?;
    dict_class.define_method("inspect", method!(RbDictionary::inspect, 0))?;

    let user_dict_class = module.define_class("UserDictionary", ruby.class_object())?;
    user_dict_class.define_method("to_s", method!(RbUserDictionary::to_s, 0))?;
    user_dict_class.define_method("inspect", method!(RbUserDictionary::inspect, 0))?;

    module.define_module_function("load_dictionary", function!(load_dictionary, 1))?;
    module.define_module_function("load_user_dictionary", function!(load_user_dictionary, 2))?;
    module.define_module_function("build_dictionary", function!(build_dictionary, 3))?;
    module.define_module_function("build_user_dictionary", function!(build_user_dictionary, 4))?;

    Ok(())
}
