//! Shared dictionary-metadata defaults and schema wiring for the bindings.
//!
//! The Python/PHP/Ruby/Node.js `Metadata` wrappers each hard-coded the same
//! default values (name `"default"`, encoding `"UTF-8"`, word cost `-10000`,
//! context ids `1288`, field value `"*"`, three `false` flags) and the same
//! default user-dictionary schema (`surface`/`reading`/`pronunciation`). This
//! module collects that into a single [`CoreMetadata`] the bindings can wrap.

use lindera::dictionary::Metadata;

use crate::schema::CoreSchema;

/// Default dictionary name.
const DEFAULT_NAME: &str = "default";
/// Default character encoding.
const DEFAULT_ENCODING: &str = "UTF-8";
/// Default cost assigned to simple user-dictionary entries.
const DEFAULT_WORD_COST: i16 = -10000;
/// Default left context id for simple user-dictionary entries.
const DEFAULT_LEFT_CONTEXT_ID: u16 = 1288;
/// Default right context id for simple user-dictionary entries.
const DEFAULT_RIGHT_CONTEXT_ID: u16 = 1288;
/// Default value substituted for missing fields.
const DEFAULT_FIELD_VALUE: &str = "*";

/// Returns the default user-dictionary schema (`surface`/`reading`/`pronunciation`).
fn default_user_dictionary_schema() -> CoreSchema {
    CoreSchema::new(vec![
        "surface".to_string(),
        "reading".to_string(),
        "pronunciation".to_string(),
    ])
}

/// Dictionary metadata shared by the bindings.
///
/// Mirrors the binding `Metadata` wrappers as a pure-Rust type (using
/// [`CoreSchema`] for the two schema fields) so each binding can wrap a
/// `CoreMetadata` instead of re-declaring the same defaults and conversions.
/// Converts to and from [`lindera::dictionary::Metadata`]; the optional
/// `model_info` carried by the lindera type is not retained (the bindings do
/// not expose it).
#[derive(Debug, Clone)]
pub struct CoreMetadata {
    /// Dictionary name.
    pub name: String,
    /// Character encoding.
    pub encoding: String,
    /// Default word cost for simple user-dictionary entries.
    pub default_word_cost: i16,
    /// Default left context id for simple user-dictionary entries.
    pub default_left_context_id: u16,
    /// Default right context id for simple user-dictionary entries.
    pub default_right_context_id: u16,
    /// Default value substituted for missing fields.
    pub default_field_value: String,
    /// Whether CSV columns are handled flexibly.
    pub flexible_csv: bool,
    /// Whether entries with invalid cost or id are skipped.
    pub skip_invalid_cost_or_id: bool,
    /// Whether morphological details are normalized.
    pub normalize_details: bool,
    /// Schema for the main dictionary.
    pub dictionary_schema: CoreSchema,
    /// Schema for the user dictionary.
    pub user_dictionary_schema: CoreSchema,
}

impl CoreMetadata {
    /// Creates metadata, falling back to the binding defaults for any `None`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Option<String>,
        encoding: Option<String>,
        default_word_cost: Option<i16>,
        default_left_context_id: Option<u16>,
        default_right_context_id: Option<u16>,
        default_field_value: Option<String>,
        flexible_csv: Option<bool>,
        skip_invalid_cost_or_id: Option<bool>,
        normalize_details: Option<bool>,
        dictionary_schema: Option<CoreSchema>,
        user_dictionary_schema: Option<CoreSchema>,
    ) -> Self {
        Self {
            name: name.unwrap_or_else(|| DEFAULT_NAME.to_string()),
            encoding: encoding.unwrap_or_else(|| DEFAULT_ENCODING.to_string()),
            default_word_cost: default_word_cost.unwrap_or(DEFAULT_WORD_COST),
            default_left_context_id: default_left_context_id.unwrap_or(DEFAULT_LEFT_CONTEXT_ID),
            default_right_context_id: default_right_context_id.unwrap_or(DEFAULT_RIGHT_CONTEXT_ID),
            default_field_value: default_field_value
                .unwrap_or_else(|| DEFAULT_FIELD_VALUE.to_string()),
            flexible_csv: flexible_csv.unwrap_or(false),
            skip_invalid_cost_or_id: skip_invalid_cost_or_id.unwrap_or(false),
            normalize_details: normalize_details.unwrap_or(false),
            dictionary_schema: dictionary_schema.unwrap_or_else(CoreSchema::create_default),
            user_dictionary_schema: user_dictionary_schema
                .unwrap_or_else(default_user_dictionary_schema),
        }
    }

    /// Creates metadata with all binding defaults.
    pub fn create_default() -> Self {
        Self::new(
            None, None, None, None, None, None, None, None, None, None, None,
        )
    }
}

impl Default for CoreMetadata {
    /// Returns [`CoreMetadata::create_default`].
    fn default() -> Self {
        Self::create_default()
    }
}

impl From<Metadata> for CoreMetadata {
    /// Converts a lindera [`Metadata`] into a [`CoreMetadata`] (dropping `model_info`).
    fn from(metadata: Metadata) -> Self {
        Self {
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

impl From<CoreMetadata> for Metadata {
    /// Converts a [`CoreMetadata`] into a lindera [`Metadata`] (`model_info` is `None`).
    fn from(metadata: CoreMetadata) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_default_uses_binding_defaults() {
        let m = CoreMetadata::create_default();
        assert_eq!(m.name, "default");
        assert_eq!(m.encoding, "UTF-8");
        assert_eq!(m.default_word_cost, -10000);
        assert_eq!(m.default_left_context_id, 1288);
        assert_eq!(m.default_right_context_id, 1288);
        assert_eq!(m.default_field_value, "*");
        assert!(!m.flexible_csv);
        assert!(!m.skip_invalid_cost_or_id);
        assert!(!m.normalize_details);
        // The default dictionary schema uses the unified `pos_detail_*` names.
        assert_eq!(m.dictionary_schema.field_count(), 13);
        assert_eq!(m.dictionary_schema.fields()[5], "pos_detail_1");
        assert_eq!(m.user_dictionary_schema.fields().len(), 3);
        assert_eq!(m.user_dictionary_schema.fields()[1], "reading");
    }

    #[test]
    fn new_applies_overrides() {
        let m = CoreMetadata::new(
            Some("custom".to_string()),
            Some("EUC-JP".to_string()),
            Some(-5000),
            Some(100),
            Some(200),
            Some("N/A".to_string()),
            Some(true),
            Some(true),
            Some(true),
            None,
            None,
        );
        assert_eq!(m.name, "custom");
        assert_eq!(m.encoding, "EUC-JP");
        assert_eq!(m.default_word_cost, -5000);
        assert_eq!(m.default_left_context_id, 100);
        assert_eq!(m.default_right_context_id, 200);
        assert_eq!(m.default_field_value, "N/A");
        assert!(m.flexible_csv && m.skip_invalid_cost_or_id && m.normalize_details);
    }

    #[test]
    fn converts_to_and_from_lindera() {
        let m = CoreMetadata::create_default();
        let lindera: Metadata = m.into();
        assert_eq!(lindera.name, "default");
        assert_eq!(lindera.default_left_context_id, 1288);
        assert_eq!(lindera.user_dictionary_schema.field_count(), 3);

        let back: CoreMetadata = lindera.into();
        assert_eq!(back.name, "default");
        assert_eq!(back.dictionary_schema.fields()[5], "pos_detail_1");
    }
}
