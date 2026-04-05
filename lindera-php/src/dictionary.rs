//! Dictionary management for morphological analysis in PHP.
//!
//! This module provides functionality for building, loading, and managing dictionaries
//! used in morphological analysis.

use std::path::Path;

use ext_php_rs::prelude::*;

use lindera::dictionary::{
    Dictionary, DictionaryBuilder, Metadata, UserDictionary,
    load_dictionary as lindera_load_dictionary,
    load_user_dictionary as lindera_load_user_dictionary,
};

use crate::error::lindera_value_err;
use crate::metadata::PhpMetadata;

/// A morphological analysis dictionary.
///
/// Contains the data structures needed for tokenization and morphological analysis.
#[php_class]
#[php(name = "Lindera\\Dictionary")]
pub struct PhpDictionary {
    /// The inner Lindera dictionary.
    pub inner: Dictionary,
}

#[php_impl]
impl PhpDictionary {
    /// Returns the version of lindera-php.
    ///
    /// # Returns
    ///
    /// The version string.
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Loads a dictionary from the specified URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - URI to the dictionary (file path or embedded name like "ipadic").
    ///
    /// # Returns
    ///
    /// A loaded Dictionary object.
    pub fn load(uri: String) -> PhpResult<PhpDictionary> {
        lindera_load_dictionary(&uri)
            .map_err(|e| lindera_value_err(format!("Failed to load dictionary from '{uri}': {e}")))
            .map(PhpDictionary::new)
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
    pub fn load_user(uri: String, metadata: &PhpMetadata) -> PhpResult<PhpUserDictionary> {
        let meta: Metadata = metadata.clone().into();
        lindera_load_user_dictionary(&uri, &meta)
            .map_err(|e| {
                lindera_value_err(format!("Failed to load user dictionary from '{uri}': {e}"))
            })
            .map(PhpUserDictionary::new)
    }

    /// Builds a dictionary from source files.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Directory containing dictionary source files.
    /// * `output_dir` - Directory where the built dictionary will be saved.
    /// * `metadata` - Metadata configuration for the dictionary.
    pub fn build(input_dir: String, output_dir: String, metadata: &PhpMetadata) -> PhpResult<()> {
        let input_path = Path::new(&input_dir);
        let output_path = Path::new(&output_dir);

        if !input_path.exists() {
            return Err(lindera_value_err(format!(
                "Input directory does not exist: {input_dir}"
            )));
        }

        let meta: Metadata = metadata.clone().into();
        let builder = DictionaryBuilder::new(meta);

        builder
            .build_dictionary(input_path, output_path)
            .map_err(|e| lindera_value_err(format!("Failed to build dictionary: {e}")))?;

        Ok(())
    }

    /// Builds a user dictionary from a CSV file.
    ///
    /// # Arguments
    ///
    /// * `kind` - Dictionary kind (reserved for future use).
    /// * `input_file` - Path to the CSV file.
    /// * `output_dir` - Directory where the built user dictionary will be saved.
    /// * `metadata` - Optional metadata configuration.
    pub fn build_user(
        _kind: String,
        input_file: String,
        output_dir: String,
        metadata: Option<&PhpMetadata>,
    ) -> PhpResult<()> {
        let input_path = Path::new(&input_file);
        let output_path = Path::new(&output_dir);

        if !input_path.exists() {
            return Err(lindera_value_err(format!(
                "Input file does not exist: {input_file}"
            )));
        }

        let meta = match metadata {
            Some(m) => {
                let lindera_meta: Metadata = m.clone().into();
                lindera_meta
            }
            None => Metadata::default(),
        };

        let builder = DictionaryBuilder::new(meta);

        builder
            .build_user_dictionary(input_path, output_path)
            .map_err(|e| lindera_value_err(format!("Failed to build user dictionary: {e}")))?;

        Ok(())
    }

    /// Returns the name of the dictionary metadata.
    ///
    /// # Returns
    ///
    /// The dictionary name.
    pub fn metadata_name(&self) -> String {
        self.inner.metadata.name.clone()
    }

    /// Returns the character encoding of the dictionary.
    ///
    /// # Returns
    ///
    /// The encoding string.
    pub fn metadata_encoding(&self) -> String {
        self.inner.metadata.encoding.clone()
    }

    /// Returns the full metadata object of the dictionary.
    ///
    /// # Returns
    ///
    /// A Metadata instance.
    pub fn metadata(&self) -> PhpMetadata {
        PhpMetadata::from(self.inner.metadata.clone())
    }

    /// Returns a string representation.
    ///
    /// # Returns
    ///
    /// The string "Dictionary".
    pub fn __to_string(&self) -> String {
        "Dictionary".to_string()
    }
}

impl PhpDictionary {
    /// Creates a new PhpDictionary from a Lindera Dictionary.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - The Lindera Dictionary to wrap.
    ///
    /// # Returns
    ///
    /// A new PhpDictionary instance.
    pub fn new(dictionary: Dictionary) -> Self {
        Self { inner: dictionary }
    }
}

/// A user-defined dictionary for custom words.
///
/// User dictionaries allow you to add custom words and their morphological features
/// that are not present in the main dictionary.
#[php_class]
#[php(name = "Lindera\\UserDictionary")]
pub struct PhpUserDictionary {
    /// The inner Lindera user dictionary.
    pub inner: UserDictionary,
}

#[php_impl]
impl PhpUserDictionary {
    /// Returns a string representation.
    ///
    /// # Returns
    ///
    /// The string "UserDictionary".
    pub fn __to_string(&self) -> String {
        "UserDictionary".to_string()
    }
}

impl PhpUserDictionary {
    /// Creates a new PhpUserDictionary from a Lindera UserDictionary.
    ///
    /// # Arguments
    ///
    /// * `user_dictionary` - The Lindera UserDictionary to wrap.
    ///
    /// # Returns
    ///
    /// A new PhpUserDictionary instance.
    pub fn new(user_dictionary: UserDictionary) -> Self {
        Self {
            inner: user_dictionary,
        }
    }
}
