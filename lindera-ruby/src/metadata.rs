//! Dictionary metadata configuration.
//!
//! This module provides structures for configuring dictionary metadata, including
//! character encodings and schema definitions. The defaults and schema wiring are
//! delegated to [`lindera_binding_core::CoreMetadata`]; this module only adds the
//! magnus wrappers.

use std::collections::HashMap;

use magnus::prelude::*;
use magnus::{Error, Ruby, function, method};

use lindera::dictionary::Metadata;
use lindera_binding_core::CoreMetadata;

/// Dictionary metadata configuration.
///
/// A thin magnus wrapper over [`lindera_binding_core::CoreMetadata`], which owns
/// the default values and the schema wiring.
#[magnus::wrap(class = "Lindera::Metadata", free_immediately, size)]
#[derive(Debug, Clone)]
pub struct RbMetadata {
    /// The backing binding-core metadata.
    inner: CoreMetadata,
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
            inner: CoreMetadata::new(
                name,
                encoding,
                default_word_cost,
                default_left_context_id,
                default_right_context_id,
                default_field_value,
                flexible_csv,
                skip_invalid_cost_or_id,
                normalize_details,
                None,
                None,
            ),
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
        self.inner.name.clone()
    }

    /// Returns the character encoding.
    fn encoding(&self) -> String {
        self.inner.encoding.clone()
    }

    /// Returns the default word cost.
    fn default_word_cost(&self) -> i16 {
        self.inner.default_word_cost
    }

    /// Returns the default left context ID.
    fn default_left_context_id(&self) -> u16 {
        self.inner.default_left_context_id
    }

    /// Returns the default right context ID.
    fn default_right_context_id(&self) -> u16 {
        self.inner.default_right_context_id
    }

    /// Returns the default field value.
    fn default_field_value(&self) -> String {
        self.inner.default_field_value.clone()
    }

    /// Returns whether flexible CSV parsing is enabled.
    fn flexible_csv(&self) -> bool {
        self.inner.flexible_csv
    }

    /// Returns whether invalid cost/ID entries should be skipped.
    fn skip_invalid_cost_or_id(&self) -> bool {
        self.inner.skip_invalid_cost_or_id
    }

    /// Returns whether morphological details should be normalized.
    fn normalize_details(&self) -> bool {
        self.inner.normalize_details
    }

    /// Converts the metadata to a Ruby hash.
    ///
    /// # Returns
    ///
    /// A HashMap of metadata properties.
    fn to_hash(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), self.inner.name.clone());
        dict.insert("encoding".to_string(), self.inner.encoding.clone());
        dict.insert(
            "default_word_cost".to_string(),
            self.inner.default_word_cost.to_string(),
        );
        dict.insert(
            "default_left_context_id".to_string(),
            self.inner.default_left_context_id.to_string(),
        );
        dict.insert(
            "default_right_context_id".to_string(),
            self.inner.default_right_context_id.to_string(),
        );
        dict.insert(
            "default_field_value".to_string(),
            self.inner.default_field_value.clone(),
        );
        dict.insert(
            "flexible_csv".to_string(),
            self.inner.flexible_csv.to_string(),
        );
        dict.insert(
            "skip_invalid_cost_or_id".to_string(),
            self.inner.skip_invalid_cost_or_id.to_string(),
        );
        dict.insert(
            "normalize_details".to_string(),
            self.inner.normalize_details.to_string(),
        );
        dict.insert(
            "dictionary_schema_fields".to_string(),
            self.inner.dictionary_schema.fields().join(","),
        );
        dict.insert(
            "user_dictionary_schema_fields".to_string(),
            self.inner.user_dictionary_schema.fields().join(","),
        );
        dict
    }

    /// Returns the string representation of the metadata.
    fn to_s(&self) -> String {
        format!(
            "Metadata(name='{}', encoding='{}')",
            self.inner.name, self.inner.encoding,
        )
    }

    /// Returns the inspect representation of the metadata.
    fn inspect(&self) -> String {
        format!(
            "#<Lindera::Metadata: name='{}', encoding='{}', schema_fields={}>",
            self.inner.name,
            self.inner.encoding,
            self.inner.dictionary_schema.field_count()
        )
    }
}

impl From<RbMetadata> for Metadata {
    fn from(metadata: RbMetadata) -> Self {
        metadata.inner.into()
    }
}

impl From<Metadata> for RbMetadata {
    fn from(metadata: Metadata) -> Self {
        RbMetadata {
            inner: CoreMetadata::from(metadata),
        }
    }
}

/// Defines Metadata class in the given Ruby module.
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
    let metadata_class = module.define_class("Metadata", ruby.class_object())?;
    metadata_class.define_singleton_method("new", function!(RbMetadata::new, 9))?;
    metadata_class
        .define_singleton_method("create_default", function!(RbMetadata::create_default, 0))?;
    metadata_class
        .define_singleton_method("from_json_file", function!(RbMetadata::from_json_file, 1))?;
    metadata_class.define_method("name", method!(RbMetadata::name, 0))?;
    metadata_class.define_method("encoding", method!(RbMetadata::encoding, 0))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use lindera_binding_core::CoreSchema;

    #[test]
    fn test_rb_metadata_to_lindera_metadata() {
        let rb_metadata = RbMetadata {
            inner: CoreMetadata::new(
                Some("test_dict".to_string()),
                Some("EUC-JP".to_string()),
                Some(-5000),
                Some(100),
                Some(200),
                Some("N/A".to_string()),
                Some(true),
                Some(true),
                Some(true),
                Some(CoreSchema::new(vec![
                    "surface".to_string(),
                    "cost".to_string(),
                ])),
                Some(CoreSchema::new(vec!["surface".to_string()])),
            ),
        };

        let lindera_metadata: Metadata = rb_metadata.into();
        assert_eq!(lindera_metadata.name, "test_dict");
        assert_eq!(lindera_metadata.encoding, "EUC-JP");
        assert_eq!(lindera_metadata.default_word_cost, -5000);
        assert_eq!(lindera_metadata.default_left_context_id, 100);
        assert_eq!(lindera_metadata.default_right_context_id, 200);
        assert_eq!(lindera_metadata.default_field_value, "N/A");
        assert!(lindera_metadata.flexible_csv);
        assert!(lindera_metadata.skip_invalid_cost_or_id);
        assert!(lindera_metadata.normalize_details);
        assert_eq!(lindera_metadata.dictionary_schema.get_all_fields().len(), 2);
        assert_eq!(
            lindera_metadata
                .user_dictionary_schema
                .get_all_fields()
                .len(),
            1
        );
    }

    #[test]
    fn test_lindera_metadata_to_rb_metadata() {
        let dict_schema =
            lindera::dictionary::Schema::new(vec!["surface".to_string(), "cost".to_string()]);
        let user_schema =
            lindera::dictionary::Schema::new(vec!["surface".to_string(), "reading".to_string()]);

        let lindera_metadata = Metadata::new(
            "my_dict".to_string(),
            "UTF-8".to_string(),
            -8000,
            500,
            600,
            "?".to_string(),
            false,
            true,
            false,
            dict_schema,
            user_schema,
        );

        let rb_metadata: RbMetadata = lindera_metadata.into();
        assert_eq!(rb_metadata.name(), "my_dict");
        assert_eq!(rb_metadata.encoding(), "UTF-8");
        assert_eq!(rb_metadata.default_word_cost(), -8000);
        assert_eq!(rb_metadata.default_left_context_id(), 500);
        assert_eq!(rb_metadata.default_right_context_id(), 600);
        assert_eq!(rb_metadata.default_field_value(), "?");
        assert!(!rb_metadata.flexible_csv());
        assert!(rb_metadata.skip_invalid_cost_or_id());
        assert!(!rb_metadata.normalize_details());
        assert_eq!(rb_metadata.inner.dictionary_schema.field_count(), 2);
        assert_eq!(rb_metadata.inner.user_dictionary_schema.field_count(), 2);
    }

    #[test]
    fn test_rb_metadata_defaults() {
        let rb_metadata = RbMetadata::create_default();
        assert_eq!(rb_metadata.name(), "default");
        assert_eq!(rb_metadata.encoding(), "UTF-8");
        assert_eq!(rb_metadata.default_word_cost(), -10000);
        assert_eq!(rb_metadata.default_left_context_id(), 1288);
        assert_eq!(rb_metadata.default_right_context_id(), 1288);
        assert_eq!(rb_metadata.default_field_value(), "*");
        assert!(!rb_metadata.flexible_csv());
        assert_eq!(rb_metadata.inner.dictionary_schema.field_count(), 13);
        assert_eq!(rb_metadata.inner.user_dictionary_schema.field_count(), 3);
    }

    #[test]
    fn test_rb_metadata_roundtrip() {
        let rb_metadata = RbMetadata::create_default();
        let lindera: Metadata = rb_metadata.into();
        let back: RbMetadata = lindera.into();
        assert_eq!(back.name(), "default");
        assert_eq!(back.encoding(), "UTF-8");
        assert_eq!(back.default_word_cost(), -10000);
        assert_eq!(back.default_left_context_id(), 1288);
        assert_eq!(back.default_right_context_id(), 1288);
        assert_eq!(back.default_field_value(), "*");
        assert_eq!(back.inner.dictionary_schema.field_count(), 13);
        assert_eq!(back.inner.user_dictionary_schema.field_count(), 3);
    }
}
