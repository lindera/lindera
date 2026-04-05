//! Dictionary metadata configuration for PHP.
//!
//! This module provides structures for configuring dictionary metadata, including
//! character encodings and schema definitions.

use std::collections::HashMap;

use ext_php_rs::prelude::*;

use lindera::dictionary::Metadata;

use crate::error::lindera_value_err;
use crate::schema::PhpSchema;

/// Dictionary metadata configuration.
///
/// Contains all configuration parameters for building and using dictionaries.
#[php_class]
#[php(name = "Lindera\\Metadata")]
pub struct PhpMetadata {
    /// Dictionary name.
    name: String,
    /// Character encoding.
    encoding: String,
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
    dictionary_schema_fields: Vec<String>,
    /// Schema for user dictionary.
    user_dictionary_schema_fields: Vec<String>,
}

#[php_impl]
impl PhpMetadata {
    /// Creates a new Metadata instance with optional parameters.
    ///
    /// All parameters are optional and use sensible defaults if not provided.
    ///
    /// # Arguments
    ///
    /// * `name` - Dictionary name (default: "default").
    /// * `encoding` - Character encoding (default: "UTF-8").
    /// * `default_word_cost` - Default word cost (default: -10000).
    /// * `default_left_context_id` - Default left context ID (default: 1288).
    /// * `default_right_context_id` - Default right context ID (default: 1288).
    /// * `default_field_value` - Default field value (default: "*").
    /// * `flexible_csv` - Allow flexible CSV (default: false).
    /// * `skip_invalid_cost_or_id` - Skip invalid entries (default: false).
    /// * `normalize_details` - Normalize details (default: false).
    ///
    /// # Returns
    ///
    /// A new Metadata instance.
    #[allow(clippy::too_many_arguments)]
    pub fn __construct(
        name: Option<String>,
        encoding: Option<String>,
        default_word_cost: Option<i64>,
        default_left_context_id: Option<i64>,
        default_right_context_id: Option<i64>,
        default_field_value: Option<String>,
        flexible_csv: Option<bool>,
        skip_invalid_cost_or_id: Option<bool>,
        normalize_details: Option<bool>,
    ) -> Self {
        let dict_schema = PhpSchema::create_default();
        let user_schema = PhpSchema::__construct(vec![
            "surface".to_string(),
            "reading".to_string(),
            "pronunciation".to_string(),
        ]);

        Self {
            name: name.unwrap_or_else(|| "default".to_string()),
            encoding: encoding.unwrap_or_else(|| "UTF-8".to_string()),
            default_word_cost: default_word_cost.unwrap_or(-10000) as i16,
            default_left_context_id: default_left_context_id.unwrap_or(1288) as u16,
            default_right_context_id: default_right_context_id.unwrap_or(1288) as u16,
            default_field_value: default_field_value.unwrap_or_else(|| "*".to_string()),
            flexible_csv: flexible_csv.unwrap_or(false),
            skip_invalid_cost_or_id: skip_invalid_cost_or_id.unwrap_or(false),
            normalize_details: normalize_details.unwrap_or(false),
            dictionary_schema_fields: dict_schema.fields,
            user_dictionary_schema_fields: user_schema.fields,
        }
    }

    /// Creates a Metadata with default values.
    ///
    /// # Returns
    ///
    /// A new default Metadata instance.
    pub fn create_default() -> Self {
        Self::__construct(None, None, None, None, None, None, None, None, None)
    }

    /// Loads Metadata from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON metadata file.
    ///
    /// # Returns
    ///
    /// A Metadata instance loaded from the file.
    pub fn from_json_file(path: String) -> PhpResult<Self> {
        let json_str = std::fs::read_to_string(&path)
            .map_err(|e| lindera_value_err(format!("Failed to read file: {e}")))?;

        let metadata: Metadata = serde_json::from_str(&json_str)
            .map_err(|e| lindera_value_err(format!("Failed to parse JSON: {e}")))?;

        Ok(Self::from(metadata))
    }

    /// Returns the dictionary name.
    ///
    /// # Returns
    ///
    /// The name string.
    #[php(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the character encoding.
    ///
    /// # Returns
    ///
    /// The encoding string.
    #[php(getter)]
    pub fn encoding(&self) -> String {
        self.encoding.clone()
    }

    /// Returns the default word cost.
    ///
    /// # Returns
    ///
    /// The cost value.
    #[php(getter)]
    pub fn default_word_cost(&self) -> i64 {
        self.default_word_cost as i64
    }

    /// Returns the default left context ID.
    ///
    /// # Returns
    ///
    /// The context ID.
    #[php(getter)]
    pub fn default_left_context_id(&self) -> i64 {
        self.default_left_context_id as i64
    }

    /// Returns the default right context ID.
    ///
    /// # Returns
    ///
    /// The context ID.
    #[php(getter)]
    pub fn default_right_context_id(&self) -> i64 {
        self.default_right_context_id as i64
    }

    /// Returns the default field value.
    ///
    /// # Returns
    ///
    /// The default value string.
    #[php(getter)]
    pub fn default_field_value(&self) -> String {
        self.default_field_value.clone()
    }

    /// Returns whether flexible CSV parsing is enabled.
    ///
    /// # Returns
    ///
    /// True if flexible CSV is enabled.
    #[php(getter)]
    pub fn flexible_csv(&self) -> bool {
        self.flexible_csv
    }

    /// Returns whether invalid cost/ID entries are skipped.
    ///
    /// # Returns
    ///
    /// True if skip is enabled.
    #[php(getter)]
    pub fn skip_invalid_cost_or_id(&self) -> bool {
        self.skip_invalid_cost_or_id
    }

    /// Returns whether details normalization is enabled.
    ///
    /// # Returns
    ///
    /// True if normalization is enabled.
    #[php(getter)]
    pub fn normalize_details(&self) -> bool {
        self.normalize_details
    }

    /// Returns the dictionary schema fields.
    ///
    /// # Returns
    ///
    /// A list of field name strings.
    #[php(getter)]
    pub fn dictionary_schema_fields(&self) -> Vec<String> {
        self.dictionary_schema_fields.clone()
    }

    /// Returns the user dictionary schema fields.
    ///
    /// # Returns
    ///
    /// A list of field name strings.
    #[php(getter)]
    pub fn user_dictionary_schema_fields(&self) -> Vec<String> {
        self.user_dictionary_schema_fields.clone()
    }

    /// Converts the metadata to an associative array.
    ///
    /// # Returns
    ///
    /// A HashMap representing the metadata.
    pub fn to_array(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), self.name.clone());
        dict.insert("encoding".to_string(), self.encoding.clone());
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
        dict
    }

    /// Returns a string representation.
    ///
    /// # Returns
    ///
    /// A string describing the metadata.
    pub fn __to_string(&self) -> String {
        format!(
            "Metadata(name='{}', encoding='{}')",
            self.name, self.encoding
        )
    }
}

impl From<PhpMetadata> for Metadata {
    fn from(metadata: PhpMetadata) -> Self {
        let dict_schema = PhpSchema::__construct(metadata.dictionary_schema_fields);
        let user_schema = PhpSchema::__construct(metadata.user_dictionary_schema_fields);

        Metadata::new(
            metadata.name,
            metadata.encoding,
            metadata.default_word_cost,
            metadata.default_left_context_id,
            metadata.default_right_context_id,
            metadata.default_field_value,
            metadata.flexible_csv,
            metadata.skip_invalid_cost_or_id,
            metadata.normalize_details,
            dict_schema.into(),
            user_schema.into(),
        )
    }
}

impl From<Metadata> for PhpMetadata {
    fn from(metadata: Metadata) -> Self {
        let dict_schema: PhpSchema = metadata.dictionary_schema.into();
        let user_schema: PhpSchema = metadata.user_dictionary_schema.into();

        PhpMetadata {
            name: metadata.name,
            encoding: metadata.encoding,
            default_word_cost: metadata.default_word_cost,
            default_left_context_id: metadata.default_left_context_id,
            default_right_context_id: metadata.default_right_context_id,
            default_field_value: metadata.default_field_value,
            flexible_csv: metadata.flexible_csv,
            skip_invalid_cost_or_id: metadata.skip_invalid_cost_or_id,
            normalize_details: metadata.normalize_details,
            dictionary_schema_fields: dict_schema.fields,
            user_dictionary_schema_fields: user_schema.fields,
        }
    }
}

impl Clone for PhpMetadata {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            encoding: self.encoding.clone(),
            default_word_cost: self.default_word_cost,
            default_left_context_id: self.default_left_context_id,
            default_right_context_id: self.default_right_context_id,
            default_field_value: self.default_field_value.clone(),
            flexible_csv: self.flexible_csv,
            skip_invalid_cost_or_id: self.skip_invalid_cost_or_id,
            normalize_details: self.normalize_details,
            dictionary_schema_fields: self.dictionary_schema_fields.clone(),
            user_dictionary_schema_fields: self.user_dictionary_schema_fields.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lindera::dictionary::Metadata;

    #[test]
    fn test_phpmetadata_default_values() {
        let meta = PhpMetadata::create_default();
        assert_eq!(meta.name, "default");
        assert_eq!(meta.encoding, "UTF-8");
        assert_eq!(meta.default_word_cost, -10000);
        assert_eq!(meta.default_left_context_id, 1288);
        assert_eq!(meta.default_right_context_id, 1288);
        assert_eq!(meta.default_field_value, "*");
        assert!(!meta.flexible_csv);
        assert!(!meta.skip_invalid_cost_or_id);
        assert!(!meta.normalize_details);
        assert_eq!(meta.dictionary_schema_fields.len(), 13);
        assert_eq!(meta.user_dictionary_schema_fields.len(), 3);
    }

    #[test]
    fn test_phpmetadata_roundtrip() {
        let meta = PhpMetadata::__construct(
            Some("test".to_string()),
            Some("UTF-8".to_string()),
            Some(-5000),
            Some(100),
            Some(200),
            Some("N/A".to_string()),
            Some(true),
            Some(true),
            Some(true),
        );
        let lindera_meta: Metadata = meta.into();
        let roundtripped: PhpMetadata = lindera_meta.into();
        assert_eq!(roundtripped.name, "test");
        assert_eq!(roundtripped.encoding, "UTF-8");
        assert_eq!(roundtripped.default_word_cost, -5000);
        assert!(roundtripped.flexible_csv);
    }
}
