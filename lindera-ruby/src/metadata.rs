//! Dictionary metadata configuration.
//!
//! This module provides structures for configuring dictionary metadata, including
//! compression algorithms, character encodings, and schema definitions.

use std::collections::HashMap;

use magnus::prelude::*;
use magnus::{Error, Ruby, function, method};

use lindera::dictionary::{CompressionAlgorithm, Metadata};

use crate::schema::RbSchema;

/// Compression algorithm for dictionary data.
///
/// Determines how dictionary data is compressed when saved to disk.
#[magnus::wrap(class = "Lindera::CompressionAlgorithm", free_immediately, size)]
#[derive(Debug, Clone)]
pub struct RbCompressionAlgorithm {
    /// Internal algorithm variant.
    inner: RbCompressionAlgorithmKind,
}

/// Internal enum for compression algorithm kind.
#[derive(Debug, Clone)]
enum RbCompressionAlgorithmKind {
    /// DEFLATE compression algorithm.
    Deflate,
    /// Zlib compression algorithm.
    Zlib,
    /// Gzip compression algorithm.
    Gzip,
    /// No compression (raw data).
    Raw,
}

impl RbCompressionAlgorithm {
    /// Returns the string representation of the compression algorithm.
    fn to_s(&self) -> &str {
        match self.inner {
            RbCompressionAlgorithmKind::Deflate => "deflate",
            RbCompressionAlgorithmKind::Zlib => "zlib",
            RbCompressionAlgorithmKind::Gzip => "gzip",
            RbCompressionAlgorithmKind::Raw => "raw",
        }
    }

    /// Returns the inspect representation of the compression algorithm.
    fn inspect(&self) -> String {
        format!("#<Lindera::CompressionAlgorithm: {:?}>", self.inner)
    }
}

impl From<RbCompressionAlgorithm> for CompressionAlgorithm {
    fn from(alg: RbCompressionAlgorithm) -> Self {
        match alg.inner {
            RbCompressionAlgorithmKind::Deflate => CompressionAlgorithm::Deflate,
            RbCompressionAlgorithmKind::Zlib => CompressionAlgorithm::Zlib,
            RbCompressionAlgorithmKind::Gzip => CompressionAlgorithm::Gzip,
            RbCompressionAlgorithmKind::Raw => CompressionAlgorithm::Raw,
        }
    }
}

impl From<CompressionAlgorithm> for RbCompressionAlgorithm {
    fn from(alg: CompressionAlgorithm) -> Self {
        let kind = match alg {
            CompressionAlgorithm::Deflate => RbCompressionAlgorithmKind::Deflate,
            CompressionAlgorithm::Zlib => RbCompressionAlgorithmKind::Zlib,
            CompressionAlgorithm::Gzip => RbCompressionAlgorithmKind::Gzip,
            CompressionAlgorithm::Raw => RbCompressionAlgorithmKind::Raw,
        };
        RbCompressionAlgorithm { inner: kind }
    }
}

/// Dictionary metadata configuration.
///
/// Contains all configuration parameters for building and using dictionaries.
#[magnus::wrap(class = "Lindera::Metadata", free_immediately, size)]
#[derive(Debug, Clone)]
pub struct RbMetadata {
    /// Dictionary name.
    name: String,
    /// Character encoding.
    encoding: String,
    /// Compression algorithm.
    compress_algorithm: RbCompressionAlgorithm,
    /// Default cost for unknown words.
    default_word_cost: i16,
    /// Default left context ID.
    default_left_context_id: u16,
    /// Default right context ID.
    default_right_context_id: u16,
    /// Default value for missing fields.
    default_field_value: String,
    /// Allow flexible CSV parsing.
    flexible_csv: bool,
    /// Skip entries with invalid cost/ID.
    skip_invalid_cost_or_id: bool,
    /// Normalize morphological details.
    normalize_details: bool,
    /// Schema for main dictionary.
    dictionary_schema: RbSchema,
    /// Schema for user dictionary.
    user_dictionary_schema: RbSchema,
}

impl RbMetadata {
    /// Creates a new `RbMetadata` with optional parameters.
    ///
    /// # Arguments
    ///
    /// All arguments are optional. Default values are used if not provided.
    ///
    /// # Returns
    ///
    /// A new `RbMetadata` instance.
    #[allow(clippy::too_many_arguments)]
    fn new(
        name: Option<String>,
        encoding: Option<String>,
        default_word_cost: Option<i16>,
        default_left_context_id: Option<u16>,
        default_right_context_id: Option<u16>,
        default_field_value: Option<String>,
        flexible_csv: Option<bool>,
        skip_invalid_cost_or_id: Option<bool>,
        normalize_details: Option<bool>,
    ) -> Self {
        RbMetadata {
            name: name.unwrap_or_else(|| "default".to_string()),
            encoding: encoding.unwrap_or_else(|| "UTF-8".to_string()),
            compress_algorithm: RbCompressionAlgorithm {
                inner: RbCompressionAlgorithmKind::Deflate,
            },
            default_word_cost: default_word_cost.unwrap_or(-10000),
            default_left_context_id: default_left_context_id.unwrap_or(1288),
            default_right_context_id: default_right_context_id.unwrap_or(1288),
            default_field_value: default_field_value.unwrap_or_else(|| "*".to_string()),
            flexible_csv: flexible_csv.unwrap_or(false),
            skip_invalid_cost_or_id: skip_invalid_cost_or_id.unwrap_or(false),
            normalize_details: normalize_details.unwrap_or(false),
            dictionary_schema: RbSchema::create_default_internal(),
            user_dictionary_schema: RbSchema::new_internal(vec![
                "surface".to_string(),
                "reading".to_string(),
                "pronunciation".to_string(),
            ]),
        }
    }

    /// Creates a default metadata instance.
    ///
    /// # Returns
    ///
    /// A new `RbMetadata` with default values.
    fn create_default() -> Self {
        RbMetadata::new(None, None, None, None, None, None, None, None, None)
    }

    /// Loads metadata from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file.
    ///
    /// # Returns
    ///
    /// A new `RbMetadata` loaded from the file.
    fn from_json_file(path: String) -> Result<Self, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");

        let json_str = std::fs::read_to_string(&path).map_err(|e| {
            Error::new(
                ruby.exception_io_error(),
                format!("Failed to read file: {e}"),
            )
        })?;

        let metadata: Metadata = serde_json::from_str(&json_str).map_err(|e| {
            Error::new(
                ruby.exception_arg_error(),
                format!("Failed to parse JSON: {e}"),
            )
        })?;

        Ok(metadata.into())
    }

    /// Returns the dictionary name.
    fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the character encoding.
    fn encoding(&self) -> String {
        self.encoding.clone()
    }

    /// Returns the compression algorithm.
    fn compress_algorithm(&self) -> RbCompressionAlgorithm {
        self.compress_algorithm.clone()
    }

    /// Returns the default word cost.
    fn default_word_cost(&self) -> i16 {
        self.default_word_cost
    }

    /// Returns the default left context ID.
    fn default_left_context_id(&self) -> u16 {
        self.default_left_context_id
    }

    /// Returns the default right context ID.
    fn default_right_context_id(&self) -> u16 {
        self.default_right_context_id
    }

    /// Returns the default field value.
    fn default_field_value(&self) -> String {
        self.default_field_value.clone()
    }

    /// Returns whether flexible CSV parsing is enabled.
    fn flexible_csv(&self) -> bool {
        self.flexible_csv
    }

    /// Returns whether invalid cost/ID entries should be skipped.
    fn skip_invalid_cost_or_id(&self) -> bool {
        self.skip_invalid_cost_or_id
    }

    /// Returns whether morphological details should be normalized.
    fn normalize_details(&self) -> bool {
        self.normalize_details
    }

    /// Converts the metadata to a Ruby hash.
    ///
    /// # Returns
    ///
    /// A HashMap of metadata properties.
    fn to_hash(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), self.name.clone());
        dict.insert("encoding".to_string(), self.encoding.clone());
        dict.insert(
            "compress_algorithm".to_string(),
            self.compress_algorithm.to_s().to_string(),
        );
        dict.insert(
            "default_word_cost".to_string(),
            self.default_word_cost.to_string(),
        );
        dict.insert(
            "default_left_context_id".to_string(),
            self.default_left_context_id.to_string(),
        );
        dict.insert(
            "default_right_context_id".to_string(),
            self.default_right_context_id.to_string(),
        );
        dict.insert(
            "default_field_value".to_string(),
            self.default_field_value.clone(),
        );
        dict.insert("flexible_csv".to_string(), self.flexible_csv.to_string());
        dict.insert(
            "skip_invalid_cost_or_id".to_string(),
            self.skip_invalid_cost_or_id.to_string(),
        );
        dict.insert(
            "normalize_details".to_string(),
            self.normalize_details.to_string(),
        );
        dict.insert(
            "dictionary_schema_fields".to_string(),
            self.dictionary_schema.fields.join(","),
        );
        dict.insert(
            "user_dictionary_schema_fields".to_string(),
            self.user_dictionary_schema.fields.join(","),
        );
        dict
    }

    /// Returns the string representation of the metadata.
    fn to_s(&self) -> String {
        format!(
            "Metadata(name='{}', encoding='{}', compress_algorithm='{}')",
            self.name,
            self.encoding,
            self.compress_algorithm.to_s()
        )
    }

    /// Returns the inspect representation of the metadata.
    fn inspect(&self) -> String {
        format!(
            "#<Lindera::Metadata: name='{}', encoding='{}', compress_algorithm={:?}, schema_fields={}>",
            self.name,
            self.encoding,
            self.compress_algorithm.inner,
            self.dictionary_schema.fields.len()
        )
    }
}

impl From<RbMetadata> for Metadata {
    fn from(metadata: RbMetadata) -> Self {
        Metadata::new(
            metadata.name,
            metadata.encoding,
            metadata.compress_algorithm.into(),
            metadata.default_word_cost,
            metadata.default_left_context_id,
            metadata.default_right_context_id,
            metadata.default_field_value,
            metadata.flexible_csv,
            metadata.skip_invalid_cost_or_id,
            metadata.normalize_details,
            metadata.dictionary_schema.into(),
            metadata.user_dictionary_schema.into(),
        )
    }
}

impl From<Metadata> for RbMetadata {
    fn from(metadata: Metadata) -> Self {
        RbMetadata {
            name: metadata.name,
            encoding: metadata.encoding,
            compress_algorithm: metadata.compress_algorithm.into(),
            default_word_cost: metadata.default_word_cost,
            default_left_context_id: metadata.default_left_context_id,
            default_right_context_id: metadata.default_right_context_id,
            default_field_value: metadata.default_field_value,
            flexible_csv: metadata.flexible_csv,
            skip_invalid_cost_or_id: metadata.skip_invalid_cost_or_id,
            normalize_details: metadata.normalize_details,
            dictionary_schema: metadata.dictionary_schema.into(),
            user_dictionary_schema: metadata.user_dictionary_schema.into(),
        }
    }
}

/// Defines Metadata and CompressionAlgorithm classes in the given Ruby module.
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
    let compression_class = module.define_class("CompressionAlgorithm", ruby.class_object())?;
    compression_class.define_method("to_s", method!(RbCompressionAlgorithm::to_s, 0))?;
    compression_class.define_method("inspect", method!(RbCompressionAlgorithm::inspect, 0))?;

    let metadata_class = module.define_class("Metadata", ruby.class_object())?;
    metadata_class.define_singleton_method("new", function!(RbMetadata::new, 9))?;
    metadata_class
        .define_singleton_method("create_default", function!(RbMetadata::create_default, 0))?;
    metadata_class
        .define_singleton_method("from_json_file", function!(RbMetadata::from_json_file, 1))?;
    metadata_class.define_method("name", method!(RbMetadata::name, 0))?;
    metadata_class.define_method("encoding", method!(RbMetadata::encoding, 0))?;
    metadata_class.define_method(
        "compress_algorithm",
        method!(RbMetadata::compress_algorithm, 0),
    )?;
    metadata_class.define_method(
        "default_word_cost",
        method!(RbMetadata::default_word_cost, 0),
    )?;
    metadata_class.define_method(
        "default_left_context_id",
        method!(RbMetadata::default_left_context_id, 0),
    )?;
    metadata_class.define_method(
        "default_right_context_id",
        method!(RbMetadata::default_right_context_id, 0),
    )?;
    metadata_class.define_method(
        "default_field_value",
        method!(RbMetadata::default_field_value, 0),
    )?;
    metadata_class.define_method("flexible_csv", method!(RbMetadata::flexible_csv, 0))?;
    metadata_class.define_method(
        "skip_invalid_cost_or_id",
        method!(RbMetadata::skip_invalid_cost_or_id, 0),
    )?;
    metadata_class.define_method(
        "normalize_details",
        method!(RbMetadata::normalize_details, 0),
    )?;
    metadata_class.define_method("to_hash", method!(RbMetadata::to_hash, 0))?;
    metadata_class.define_method("to_h", method!(RbMetadata::to_hash, 0))?;
    metadata_class.define_method("to_s", method!(RbMetadata::to_s, 0))?;
    metadata_class.define_method("inspect", method!(RbMetadata::inspect, 0))?;

    Ok(())
}
