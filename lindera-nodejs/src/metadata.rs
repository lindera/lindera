//! Dictionary metadata configuration.
//!
//! This module provides structures for configuring dictionary metadata, including
//! character encodings and schema definitions. The defaults and schema wiring are
//! delegated to [`lindera_binding::CoreMetadata`]; this module only adds the
//! napi wrappers.

use std::collections::HashMap;

use lindera::dictionary::Metadata;
use lindera_binding::CoreMetadata;

use crate::util::{integer_arg, optional_integer_arg};

/// Options for creating a Metadata instance.
///
/// All fields are optional. When omitted, default values are used.
#[napi(object)]
pub struct MetadataOptions {
    /// Dictionary name (default: "default").
    pub name: Option<String>,
    /// Character encoding (default: "UTF-8").
    pub encoding: Option<String>,
    /// Default cost for unknown words, an integer from -32768 to 32767
    /// (default: -10000).
    pub default_word_cost: Option<f64>,
    /// Default left context ID, an integer from 0 to 65535 (default: 1288).
    pub default_left_context_id: Option<f64>,
    /// Default right context ID, an integer from 0 to 65535 (default: 1288).
    pub default_right_context_id: Option<f64>,
    /// Default value for missing fields (default: "*").
    pub default_field_value: Option<String>,
    /// Allow flexible CSV parsing (default: false).
    pub flexible_csv: Option<bool>,
    /// Skip entries with invalid cost or ID (default: false).
    pub skip_invalid_cost_or_id: Option<bool>,
}

/// Dictionary metadata configuration.
///
/// A thin napi wrapper over [`lindera_binding::CoreMetadata`], which owns
/// the default values and the schema wiring.
#[napi(js_name = "Metadata")]
pub struct JsMetadata {
    /// The backing lindera-binding metadata.
    inner: CoreMetadata,
}

#[napi]
impl JsMetadata {
    /// Creates a new Metadata with optional configuration.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional configuration object. When omitted, all defaults are used.
    ///
    /// # Returns
    ///
    /// The metadata, or an `InvalidArg` error for a cost or context ID that
    /// is not an integer in its range.
    #[napi(constructor)]
    pub fn new(options: Option<MetadataOptions>) -> napi::Result<Self> {
        let opts = options.unwrap_or(MetadataOptions {
            name: None,
            encoding: None,
            default_word_cost: None,
            default_left_context_id: None,
            default_right_context_id: None,
            default_field_value: None,
            flexible_csv: None,
            skip_invalid_cost_or_id: None,
        });

        Ok(JsMetadata {
            inner: CoreMetadata::new(
                opts.name,
                opts.encoding,
                optional_integer_arg("defaultWordCost", opts.default_word_cost)?,
                optional_integer_arg("defaultLeftContextId", opts.default_left_context_id)?,
                optional_integer_arg("defaultRightContextId", opts.default_right_context_id)?,
                opts.default_field_value,
                opts.flexible_csv,
                opts.skip_invalid_cost_or_id,
                None,
                None,
            ),
        })
    }

    /// Creates a Metadata with all default values.
    ///
    /// # Returns
    ///
    /// A Metadata instance with default configuration.
    #[napi(factory)]
    pub fn create_default() -> Self {
        JsMetadata {
            inner: CoreMetadata::new(None, None, None, None, None, None, None, None, None, None),
        }
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
        self.inner.name.clone()
    }

    /// Sets the dictionary name.
    #[napi(setter)]
    pub fn set_name(&mut self, name: String) {
        self.inner.name = name;
    }

    /// Character encoding.
    #[napi(getter)]
    pub fn encoding(&self) -> String {
        self.inner.encoding.clone()
    }

    /// Sets the character encoding.
    #[napi(setter)]
    pub fn set_encoding(&mut self, encoding: String) {
        self.inner.encoding = encoding;
    }

    /// Default word cost.
    #[napi(getter)]
    pub fn default_word_cost(&self) -> i32 {
        self.inner.default_word_cost as i32
    }

    /// Sets the default word cost.
    ///
    /// # Arguments
    ///
    /// * `cost` - An integer from -32768 to 32767.
    ///
    /// # Returns
    ///
    /// Nothing, or an `InvalidArg` error for any other number, which leaves
    /// the cost unchanged.
    #[napi(setter)]
    pub fn set_default_word_cost(&mut self, cost: f64) -> napi::Result<()> {
        self.inner.default_word_cost = integer_arg("defaultWordCost", cost)?;
        Ok(())
    }

    /// Default left context ID.
    #[napi(getter)]
    pub fn default_left_context_id(&self) -> u32 {
        self.inner.default_left_context_id as u32
    }

    /// Sets the default left context ID.
    ///
    /// # Arguments
    ///
    /// * `id` - An integer from 0 to 65535.
    ///
    /// # Returns
    ///
    /// Nothing, or an `InvalidArg` error for any other number, which leaves
    /// the ID unchanged.
    #[napi(setter)]
    pub fn set_default_left_context_id(&mut self, id: f64) -> napi::Result<()> {
        self.inner.default_left_context_id = integer_arg("defaultLeftContextId", id)?;
        Ok(())
    }

    /// Default right context ID.
    #[napi(getter)]
    pub fn default_right_context_id(&self) -> u32 {
        self.inner.default_right_context_id as u32
    }

    /// Sets the default right context ID.
    ///
    /// # Arguments
    ///
    /// * `id` - An integer from 0 to 65535.
    ///
    /// # Returns
    ///
    /// Nothing, or an `InvalidArg` error for any other number, which leaves
    /// the ID unchanged.
    #[napi(setter)]
    pub fn set_default_right_context_id(&mut self, id: f64) -> napi::Result<()> {
        self.inner.default_right_context_id = integer_arg("defaultRightContextId", id)?;
        Ok(())
    }

    /// Default field value for missing fields.
    #[napi(getter)]
    pub fn default_field_value(&self) -> String {
        self.inner.default_field_value.clone()
    }

    /// Sets the default field value.
    #[napi(setter)]
    pub fn set_default_field_value(&mut self, value: String) {
        self.inner.default_field_value = value;
    }

    /// Whether flexible CSV parsing is enabled.
    #[napi(getter)]
    pub fn flexible_csv(&self) -> bool {
        self.inner.flexible_csv
    }

    /// Sets flexible CSV parsing.
    #[napi(setter)]
    pub fn set_flexible_csv(&mut self, value: bool) {
        self.inner.flexible_csv = value;
    }

    /// Whether to skip entries with invalid cost or ID.
    #[napi(getter)]
    pub fn skip_invalid_cost_or_id(&self) -> bool {
        self.inner.skip_invalid_cost_or_id
    }

    /// Sets whether to skip invalid entries.
    #[napi(setter)]
    pub fn set_skip_invalid_cost_or_id(&mut self, value: bool) {
        self.inner.skip_invalid_cost_or_id = value;
    }

    /// Returns a plain object representation of the metadata.
    ///
    /// # Returns
    ///
    /// A HashMap containing all metadata properties as strings. When the
    /// metadata carries left-space penalty rules (as ko-dic's
    /// `metadata.json` does), they are included under `spacePenalty` as a
    /// JSON string such as `{"rules":[...]}`; otherwise the key is absent.
    #[napi]
    pub fn to_object(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), self.inner.name.clone());
        dict.insert("encoding".to_string(), self.inner.encoding.clone());
        dict.insert(
            "defaultWordCost".to_string(),
            self.inner.default_word_cost.to_string(),
        );
        dict.insert(
            "defaultLeftContextId".to_string(),
            self.inner.default_left_context_id.to_string(),
        );
        dict.insert(
            "defaultRightContextId".to_string(),
            self.inner.default_right_context_id.to_string(),
        );
        dict.insert(
            "defaultFieldValue".to_string(),
            self.inner.default_field_value.clone(),
        );
        dict.insert(
            "flexibleCsv".to_string(),
            self.inner.flexible_csv.to_string(),
        );
        dict.insert(
            "skipInvalidCostOrId".to_string(),
            self.inner.skip_invalid_cost_or_id.to_string(),
        );
        if let Some(space_penalty) = self.inner.space_penalty_json() {
            dict.insert("spacePenalty".to_string(), space_penalty);
        }
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
        metadata.inner.clone().into()
    }
}

impl From<JsMetadata> for Metadata {
    fn from(metadata: JsMetadata) -> Self {
        metadata.inner.into()
    }
}

impl From<Metadata> for JsMetadata {
    fn from(metadata: Metadata) -> Self {
        JsMetadata {
            inner: CoreMetadata::from(metadata),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_metadata_to_lindera_metadata() {
        let js_metadata = JsMetadata::new(None).unwrap();
        let lindera_metadata: Metadata = js_metadata.into();
        assert_eq!(lindera_metadata.name, "default");
        assert_eq!(lindera_metadata.encoding, "UTF-8");
        assert_eq!(lindera_metadata.default_word_cost, -10000);
        assert_eq!(lindera_metadata.default_left_context_id, 1288);
        assert_eq!(lindera_metadata.default_right_context_id, 1288);
        assert_eq!(lindera_metadata.default_field_value, "*");
        assert!(!lindera_metadata.flexible_csv);
        assert!(!lindera_metadata.skip_invalid_cost_or_id);
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
    }

    #[test]
    fn test_js_metadata_with_custom_options() {
        let opts = MetadataOptions {
            name: Some("custom".to_string()),
            encoding: Some("Shift_JIS".to_string()),
            default_word_cost: Some(-5000.0),
            default_left_context_id: Some(100.0),
            default_right_context_id: Some(200.0),
            default_field_value: Some("-".to_string()),
            flexible_csv: Some(true),
            skip_invalid_cost_or_id: Some(true),
        };
        let js_metadata = JsMetadata::new(Some(opts)).unwrap();
        assert_eq!(js_metadata.name(), "custom");
        assert_eq!(js_metadata.encoding(), "Shift_JIS");
        assert_eq!(js_metadata.default_word_cost(), -5000);
        assert_eq!(js_metadata.default_left_context_id(), 100);
        assert_eq!(js_metadata.default_right_context_id(), 200);
        assert_eq!(js_metadata.default_field_value(), "-");
        assert!(js_metadata.flexible_csv());
        assert!(js_metadata.skip_invalid_cost_or_id());
    }

    #[test]
    fn test_js_metadata_rejects_numbers_outside_the_range() {
        let options = |cost, left, right| MetadataOptions {
            name: None,
            encoding: None,
            default_word_cost: cost,
            default_left_context_id: left,
            default_right_context_id: right,
            default_field_value: None,
            flexible_csv: None,
            skip_invalid_cost_or_id: None,
        };
        let reason = |options| JsMetadata::new(Some(options)).err().map(|e| e.reason);
        assert_eq!(
            reason(options(Some(100000.0), None, None)).as_deref(),
            Some("defaultWordCost must be an integer between -32768 and 32767, got 100000")
        );
        assert_eq!(
            reason(options(Some(1.5), None, None)).as_deref(),
            Some("defaultWordCost must be an integer between -32768 and 32767, got 1.5")
        );
        assert_eq!(
            reason(options(None, Some(4294967301.0), None)).as_deref(),
            Some("defaultLeftContextId must be an integer between 0 and 65535, got 4294967301")
        );
        assert_eq!(
            reason(options(None, None, Some(-1.0))).as_deref(),
            Some("defaultRightContextId must be an integer between 0 and 65535, got -1")
        );
        assert_eq!(
            reason(options(Some(-32768.0), Some(0.0), Some(65535.0))),
            None
        );
    }

    #[test]
    fn test_js_metadata_setters_reject_numbers_outside_the_range() {
        let mut js_metadata = JsMetadata::new(None).unwrap();
        let error = js_metadata.set_default_word_cost(f64::NAN).unwrap_err();
        assert_eq!(error.status, napi::Status::InvalidArg);
        assert_eq!(
            error.reason,
            "defaultWordCost must be an integer between -32768 and 32767, got NaN"
        );
        assert!(js_metadata.set_default_left_context_id(65536.0).is_err());
        assert!(js_metadata.set_default_right_context_id(-1.0).is_err());
        // A rejected value leaves the setting unchanged.
        assert_eq!(js_metadata.default_word_cost(), -10000);
        assert_eq!(js_metadata.default_left_context_id(), 1288);
        assert_eq!(js_metadata.default_right_context_id(), 1288);

        js_metadata.set_default_word_cost(32767.0).unwrap();
        js_metadata.set_default_left_context_id(65535.0).unwrap();
        assert_eq!(js_metadata.default_word_cost(), 32767);
        assert_eq!(js_metadata.default_left_context_id(), 65535);
    }

    #[test]
    fn test_to_object_omits_space_penalty_when_absent() {
        let js_metadata = JsMetadata::new(None).unwrap();
        assert!(!js_metadata.to_object().contains_key("spacePenalty"));
    }

    #[test]
    fn test_to_object_exposes_ko_dic_space_penalty() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../lindera-ko-dic/metadata.json")
            .to_string_lossy()
            .into_owned();
        let js_metadata = JsMetadata::from_json_file(path).unwrap();
        let object = js_metadata.to_object();
        let space_penalty = object.get("spacePenalty").unwrap();
        assert!(space_penalty.contains("\"rules\""));
        assert!(space_penalty.contains("JKS"));

        // The rules survive the conversion used by `buildDictionary`.
        let lindera_metadata = JsMetadata::to_lindera_metadata(&js_metadata);
        assert!(lindera_metadata.space_penalty.is_some());
    }

    #[test]
    fn test_js_metadata_roundtrip() {
        let original = JsMetadata::new(None).unwrap();
        let lindera: Metadata = original.into();
        let roundtripped: JsMetadata = lindera.into();
        assert_eq!(roundtripped.name(), "default");
        assert_eq!(roundtripped.encoding(), "UTF-8");
        assert_eq!(roundtripped.default_word_cost(), -10000);
        assert_eq!(roundtripped.default_left_context_id(), 1288);
        assert_eq!(roundtripped.default_right_context_id(), 1288);
    }
}
