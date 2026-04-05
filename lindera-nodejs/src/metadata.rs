//! Dictionary metadata configuration.
//!
//! This module provides structures for configuring dictionary metadata, including
//! character encodings and schema definitions.

use std::collections::HashMap;

use lindera::dictionary::Metadata;

use crate::schema::JsSchema;

/// Options for creating a Metadata instance.
///
/// All fields are optional. When omitted, default values are used.
#[napi(object)]
pub struct MetadataOptions {
    /// Dictionary name (default: "default").
    pub name: Option<String>,
    /// Character encoding (default: "UTF-8").
    pub encoding: Option<String>,
    /// Default cost for unknown words (default: -10000).
    pub default_word_cost: Option<i32>,
    /// Default left context ID (default: 1288).
    pub default_left_context_id: Option<u32>,
    /// Default right context ID (default: 1288).
    pub default_right_context_id: Option<u32>,
    /// Default value for missing fields (default: "*").
    pub default_field_value: Option<String>,
    /// Allow flexible CSV parsing (default: false).
    pub flexible_csv: Option<bool>,
    /// Skip entries with invalid cost or ID (default: false).
    pub skip_invalid_cost_or_id: Option<bool>,
    /// Normalize morphological details (default: false).
    pub normalize_details: Option<bool>,
}

/// Dictionary metadata configuration.
///
/// Contains all configuration parameters for building and using dictionaries.
#[napi(js_name = "Metadata")]
pub struct JsMetadata {
    name: String,
    encoding: String,
    default_word_cost: i16,
    default_left_context_id: u16,
    default_right_context_id: u16,
    default_field_value: String,
    flexible_csv: bool,
    skip_invalid_cost_or_id: bool,
    normalize_details: bool,
    dictionary_schema: JsSchema,
    user_dictionary_schema: JsSchema,
}

#[napi]
impl JsMetadata {
    /// Creates a new Metadata with optional configuration.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional configuration object. When omitted, all defaults are used.
    #[napi(constructor)]
    pub fn new(options: Option<MetadataOptions>) -> Self {
        let opts = options.unwrap_or(MetadataOptions {
            name: None,
            encoding: None,
            default_word_cost: None,
            default_left_context_id: None,
            default_right_context_id: None,
            default_field_value: None,
            flexible_csv: None,
            skip_invalid_cost_or_id: None,
            normalize_details: None,
        });

        JsMetadata {
            name: opts.name.unwrap_or_else(|| "default".to_string()),
            encoding: opts.encoding.unwrap_or_else(|| "UTF-8".to_string()),
            default_word_cost: opts.default_word_cost.unwrap_or(-10000) as i16,
            default_left_context_id: opts.default_left_context_id.unwrap_or(1288) as u16,
            default_right_context_id: opts.default_right_context_id.unwrap_or(1288) as u16,
            default_field_value: opts.default_field_value.unwrap_or_else(|| "*".to_string()),
            flexible_csv: opts.flexible_csv.unwrap_or(false),
            skip_invalid_cost_or_id: opts.skip_invalid_cost_or_id.unwrap_or(false),
            normalize_details: opts.normalize_details.unwrap_or(false),
            dictionary_schema: JsSchema::create_default(),
            user_dictionary_schema: JsSchema::new(vec![
                "surface".to_string(),
                "reading".to_string(),
                "pronunciation".to_string(),
            ]),
        }
    }

    /// Creates a Metadata with all default values.
    ///
    /// # Returns
    ///
    /// A Metadata instance with default configuration.
    #[napi(factory)]
    pub fn create_default() -> Self {
        JsMetadata::new(None)
    }

    /// Loads metadata from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON metadata file.
    ///
    /// # Returns
    ///
    /// A Metadata instance loaded from the file.
    #[napi(factory)]
    pub fn from_json_file(path: String) -> napi::Result<Self> {
        let json_str = std::fs::read_to_string(&path).map_err(|e| {
            napi::Error::new(
                napi::Status::GenericFailure,
                format!("Failed to read file: {e}"),
            )
        })?;

        let metadata: Metadata = serde_json::from_str(&json_str).map_err(|e| {
            napi::Error::new(
                napi::Status::GenericFailure,
                format!("Failed to parse JSON: {e}"),
            )
        })?;

        Ok(metadata.into())
    }

    /// Dictionary name.
    #[napi(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Sets the dictionary name.
    #[napi(setter)]
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Character encoding.
    #[napi(getter)]
    pub fn encoding(&self) -> String {
        self.encoding.clone()
    }

    /// Sets the character encoding.
    #[napi(setter)]
    pub fn set_encoding(&mut self, encoding: String) {
        self.encoding = encoding;
    }

    /// Default word cost.
    #[napi(getter)]
    pub fn default_word_cost(&self) -> i32 {
        self.default_word_cost as i32
    }

    /// Sets the default word cost.
    #[napi(setter)]
    pub fn set_default_word_cost(&mut self, cost: i32) {
        self.default_word_cost = cost as i16;
    }

    /// Default left context ID.
    #[napi(getter)]
    pub fn default_left_context_id(&self) -> u32 {
        self.default_left_context_id as u32
    }

    /// Sets the default left context ID.
    #[napi(setter)]
    pub fn set_default_left_context_id(&mut self, id: u32) {
        self.default_left_context_id = id as u16;
    }

    /// Default right context ID.
    #[napi(getter)]
    pub fn default_right_context_id(&self) -> u32 {
        self.default_right_context_id as u32
    }

    /// Sets the default right context ID.
    #[napi(setter)]
    pub fn set_default_right_context_id(&mut self, id: u32) {
        self.default_right_context_id = id as u16;
    }

    /// Default field value for missing fields.
    #[napi(getter)]
    pub fn default_field_value(&self) -> String {
        self.default_field_value.clone()
    }

    /// Sets the default field value.
    #[napi(setter)]
    pub fn set_default_field_value(&mut self, value: String) {
        self.default_field_value = value;
    }

    /// Whether flexible CSV parsing is enabled.
    #[napi(getter)]
    pub fn flexible_csv(&self) -> bool {
        self.flexible_csv
    }

    /// Sets flexible CSV parsing.
    #[napi(setter)]
    pub fn set_flexible_csv(&mut self, value: bool) {
        self.flexible_csv = value;
    }

    /// Whether to skip entries with invalid cost or ID.
    #[napi(getter)]
    pub fn skip_invalid_cost_or_id(&self) -> bool {
        self.skip_invalid_cost_or_id
    }

    /// Sets whether to skip invalid entries.
    #[napi(setter)]
    pub fn set_skip_invalid_cost_or_id(&mut self, value: bool) {
        self.skip_invalid_cost_or_id = value;
    }

    /// Whether to normalize morphological details.
    #[napi(getter)]
    pub fn normalize_details(&self) -> bool {
        self.normalize_details
    }

    /// Sets whether to normalize details.
    #[napi(setter)]
    pub fn set_normalize_details(&mut self, value: bool) {
        self.normalize_details = value;
    }

    /// Returns a plain object representation of the metadata.
    ///
    /// # Returns
    ///
    /// A HashMap containing all metadata properties as strings.
    #[napi]
    pub fn to_object(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), self.name.clone());
        dict.insert("encoding".to_string(), self.encoding.clone());
        dict.insert(
            "defaultWordCost".to_string(),
            self.default_word_cost.to_string(),
        );
        dict.insert(
            "defaultLeftContextId".to_string(),
            self.default_left_context_id.to_string(),
        );
        dict.insert(
            "defaultRightContextId".to_string(),
            self.default_right_context_id.to_string(),
        );
        dict.insert(
            "defaultFieldValue".to_string(),
            self.default_field_value.clone(),
        );
        dict.insert("flexibleCsv".to_string(), self.flexible_csv.to_string());
        dict.insert(
            "skipInvalidCostOrId".to_string(),
            self.skip_invalid_cost_or_id.to_string(),
        );
        dict.insert(
            "normalizeDetails".to_string(),
            self.normalize_details.to_string(),
        );
        dict
    }
}

impl JsMetadata {
    /// Converts a reference to JsMetadata into a lindera Metadata.
    ///
    /// # Arguments
    ///
    /// * `metadata` - Reference to JsMetadata.
    ///
    /// # Returns
    ///
    /// A lindera Metadata instance.
    pub fn to_lindera_metadata(metadata: &JsMetadata) -> Metadata {
        Metadata::new(
            metadata.name.clone(),
            metadata.encoding.clone(),
            metadata.default_word_cost,
            metadata.default_left_context_id,
            metadata.default_right_context_id,
            metadata.default_field_value.clone(),
            metadata.flexible_csv,
            metadata.skip_invalid_cost_or_id,
            metadata.normalize_details,
            JsSchema::new(metadata.dictionary_schema.fields()).into(),
            JsSchema::new(metadata.user_dictionary_schema.fields()).into(),
        )
    }
}

impl From<JsMetadata> for Metadata {
    fn from(metadata: JsMetadata) -> Self {
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
            metadata.dictionary_schema.into(),
            metadata.user_dictionary_schema.into(),
        )
    }
}

impl From<Metadata> for JsMetadata {
    fn from(metadata: Metadata) -> Self {
        JsMetadata {
            name: metadata.name,
            encoding: metadata.encoding,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_metadata_to_lindera_metadata() {
        let js_metadata = JsMetadata::new(None);
        let lindera_metadata: Metadata = js_metadata.into();
        assert_eq!(lindera_metadata.name, "default");
        assert_eq!(lindera_metadata.encoding, "UTF-8");
        assert_eq!(lindera_metadata.default_word_cost, -10000);
        assert_eq!(lindera_metadata.default_left_context_id, 1288);
        assert_eq!(lindera_metadata.default_right_context_id, 1288);
        assert_eq!(lindera_metadata.default_field_value, "*");
        assert!(!lindera_metadata.flexible_csv);
        assert!(!lindera_metadata.skip_invalid_cost_or_id);
        assert!(!lindera_metadata.normalize_details);
    }

    #[test]
    fn test_lindera_metadata_to_js_metadata() {
        let lindera_metadata = Metadata::new(
            "test".to_string(),
            "EUC-JP".to_string(),
            -5000,
            100,
            200,
            "-".to_string(),
            true,
            true,
            true,
            lindera::dictionary::Schema::default(),
            lindera::dictionary::Schema::default(),
        );
        let js_metadata: JsMetadata = lindera_metadata.into();
        assert_eq!(js_metadata.name(), "test");
        assert_eq!(js_metadata.encoding(), "EUC-JP");
        assert_eq!(js_metadata.default_word_cost(), -5000);
        assert_eq!(js_metadata.default_left_context_id(), 100);
        assert_eq!(js_metadata.default_right_context_id(), 200);
        assert_eq!(js_metadata.default_field_value(), "-");
        assert!(js_metadata.flexible_csv());
        assert!(js_metadata.skip_invalid_cost_or_id());
        assert!(js_metadata.normalize_details());
    }

    #[test]
    fn test_js_metadata_with_custom_options() {
        let opts = MetadataOptions {
            name: Some("custom".to_string()),
            encoding: Some("Shift_JIS".to_string()),
            default_word_cost: Some(-5000),
            default_left_context_id: Some(100),
            default_right_context_id: Some(200),
            default_field_value: Some("-".to_string()),
            flexible_csv: Some(true),
            skip_invalid_cost_or_id: Some(true),
            normalize_details: Some(true),
        };
        let js_metadata = JsMetadata::new(Some(opts));
        assert_eq!(js_metadata.name(), "custom");
        assert_eq!(js_metadata.encoding(), "Shift_JIS");
        assert_eq!(js_metadata.default_word_cost(), -5000);
        assert_eq!(js_metadata.default_left_context_id(), 100);
        assert_eq!(js_metadata.default_right_context_id(), 200);
        assert_eq!(js_metadata.default_field_value(), "-");
        assert!(js_metadata.flexible_csv());
        assert!(js_metadata.skip_invalid_cost_or_id());
        assert!(js_metadata.normalize_details());
    }

    #[test]
    fn test_js_metadata_roundtrip() {
        let original = JsMetadata::new(None);
        let lindera: Metadata = original.into();
        let roundtripped: JsMetadata = lindera.into();
        assert_eq!(roundtripped.name(), "default");
        assert_eq!(roundtripped.encoding(), "UTF-8");
        assert_eq!(roundtripped.default_word_cost(), -10000);
        assert_eq!(roundtripped.default_left_context_id(), 1288);
        assert_eq!(roundtripped.default_right_context_id(), 1288);
    }
}
