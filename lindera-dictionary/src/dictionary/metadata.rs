use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::dictionary::context_id_map::ContextIdMap;
use crate::dictionary::schema::Schema;

const DEFAULT_WORD_COST: i16 = -10000;
const DEFAULT_LEFT_CONTEXT_ID: u16 = 1288;
const DEFAULT_RIGHT_CONTEXT_ID: u16 = 1288;
const DEFAULT_FIELD_VALUE: &str = "*";

/// On-disk layout version of a *built* dictionary directory.
///
/// Written into the built `metadata.json` by
/// [`crate::builder::DictionaryBuilder::build_metadata`] and checked by
/// [`Metadata::validate_format_version`] when a dictionary directory is
/// loaded, so that a dictionary built by an incompatible version of this crate
/// fails with a clear message instead of being silently misread. `matrix.mtx`,
/// `dict.vals` and `dict.words` are headerless raw arrays; nothing about them
/// makes a stale file detectable on its own.
///
/// # When to bump this
///
/// Bump on **any** change to the bytes of a built artifact, including:
///
/// - adding, removing or renaming a file in the dictionary directory,
/// - changing a record layout or a value encoding,
/// - upgrading a dependency whose serialized form is written verbatim
///   (`daachorse` for `dict.da`, `rkyv` for `char_def.bin` and `unk.bin`).
///
/// That last case is easy to miss: a `daachorse` major bump changes `dict.da`
/// without a single line of this crate changing, and the build cache keyed on
/// this constant is what stops a stale automaton from being served to
/// `deserialize_unchecked`.
///
/// # History
///
/// * `1` - the layout shipped through v5.x.
pub const DICTIONARY_FORMAT_VERSION: u32 = 1;

/// The format version assumed for a built dictionary whose `metadata.json`
/// predates the `format_version` field.
///
/// Dictionaries built before the field existed are exactly the v5.x layout,
/// which is version 1. Source `metadata.json` files also omit the field, but
/// they describe build *inputs* and are never format-checked -- see
/// [`Metadata::validate_format_version`].
const LEGACY_FORMAT_VERSION: u32 = 1;

/// Returns the format version to assume when `metadata.json` does not carry
/// one.
///
/// # Returns
///
/// [`LEGACY_FORMAT_VERSION`].
fn legacy_format_version() -> u32 {
    LEGACY_FORMAT_VERSION
}

#[derive(Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct ModelInfo {
    pub feature_count: usize,
    pub label_count: usize,
    pub max_left_context_id: usize,
    pub max_right_context_id: usize,
    pub connection_matrix_size: String,
    pub version: String,
    pub training_iterations: u64,
    pub regularization: f64,
    pub updated_at: u64,
}

#[derive(Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct Metadata {
    /// On-disk layout version of the dictionary directory this metadata was
    /// written into. See [`DICTIONARY_FORMAT_VERSION`].
    ///
    /// Absent from source `metadata.json` files, which describe build inputs
    /// rather than a built dictionary, and absent from dictionaries built
    /// before the field existed; both read back as
    /// [`LEGACY_FORMAT_VERSION`].
    #[serde(default = "legacy_format_version")]
    pub format_version: u32,
    pub name: String,                  // Name of the dictionary
    pub encoding: String,              // Character encoding
    pub default_word_cost: i16,        // Word cost for simple user dictionary
    pub default_left_context_id: u16,  // Context ID for simple user dictionary
    pub default_right_context_id: u16, // Context ID for simple user dictionary
    pub default_field_value: String,   // Default value for fields in simple user dictionary
    pub flexible_csv: bool,            // Handle CSV columns flexibly
    pub skip_invalid_cost_or_id: bool, // Skip invalid cost or ID
    pub normalize_details: bool,       // Normalize characters
    /// Reorder connection-cost context IDs by frequency at build time so that
    /// frequently-used connection-matrix cells cluster in cache. Optional and
    /// defaults to `false`; when `false` the field is omitted from `metadata.json`
    /// so existing files stay byte-identical, and the build output is unchanged.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub connection_id_mapping: bool,
    /// The context-ID permutation that was applied when this dictionary was built.
    ///
    /// Written into the *built* `metadata.json` when `connection_id_mapping` is on, so
    /// that anything compiled later against this dictionary — most importantly a
    /// detailed user dictionary — can be relabeled into the same ID space. Absent (and
    /// omitted from the file) for an un-remapped dictionary, which keeps those builds
    /// byte-identical. Source `metadata.json` files carry only the boolean flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id_map: Option<ContextIdMap>,
    pub dictionary_schema: Schema,      // Schema for the dictionary
    pub user_dictionary_schema: Schema, // Schema for user dictionary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_info: Option<ModelInfo>, // Training model information (optional)
}

impl Default for Metadata {
    fn default() -> Self {
        // Default metadata values can be adjusted as needed
        Metadata::new(
            "default".to_string(),
            "UTF-8".to_string(),
            DEFAULT_WORD_COST,
            DEFAULT_LEFT_CONTEXT_ID,
            DEFAULT_RIGHT_CONTEXT_ID,
            DEFAULT_FIELD_VALUE.to_string(),
            false,
            false,
            false,
            Schema::default(),
            Schema::new(vec![
                "surface".to_string(),
                "reading".to_string(),
                "pronunciation".to_string(),
            ]),
        )
    }
}

impl Metadata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        encoding: String,
        simple_word_cost: i16,
        default_left_context_id: u16,
        default_right_context_id: u16,
        default_field_value: String,
        flexible_csv: bool,
        skip_invalid_cost_or_id: bool,
        normalize_details: bool,
        schema: Schema,
        userdic_schema: Schema,
    ) -> Self {
        Self {
            format_version: DICTIONARY_FORMAT_VERSION,
            encoding,
            default_word_cost: simple_word_cost,
            default_left_context_id,
            default_right_context_id,
            default_field_value,
            dictionary_schema: schema,
            name,
            flexible_csv,
            skip_invalid_cost_or_id,
            normalize_details,
            connection_id_mapping: false,
            context_id_map: None,
            user_dictionary_schema: userdic_schema,
            model_info: None,
        }
    }

    /// Load metadata from binary data (JSON format).
    /// This provides a consistent interface with other dictionary components.
    pub fn load(data: &[u8]) -> crate::LinderaResult<Self> {
        // If data is empty, return an error since metadata is required
        if data.is_empty() {
            return Err(crate::error::LinderaErrorKind::Io
                .with_error(anyhow::anyhow!("Empty metadata data")));
        }

        // Deserialize as JSON
        serde_json::from_slice(data).map_err(|err| {
            crate::error::LinderaErrorKind::Deserialize
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to deserialize metadata from JSON")
        })
    }

    /// Rejects a built dictionary whose on-disk layout this build cannot read.
    ///
    /// Call this after loading the `metadata.json` of a *built dictionary
    /// directory*. Do **not** call it on a source `metadata.json`: those
    /// describe build inputs (schema, encoding, `flexible_csv` and friends),
    /// carry no `format_version`, and stay valid across format changes.
    ///
    /// Without this check a stale dictionary is not merely unreadable but
    /// silently wrong: `matrix.mtx`, `dict.vals` and `dict.words` are
    /// headerless raw arrays, so an old file of a plausible length decodes
    /// into garbage costs rather than failing.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the dictionary was built with this crate's format
    /// version, otherwise an error naming both versions and how to recover.
    pub fn validate_format_version(&self) -> crate::LinderaResult<()> {
        if self.format_version == DICTIONARY_FORMAT_VERSION {
            return Ok(());
        }

        let hint = if self.format_version < DICTIONARY_FORMAT_VERSION {
            "rebuild it with `lindera build`, or download a matching prebuilt dictionary with `lindera download`"
        } else {
            "upgrade Lindera to a version that understands this dictionary"
        };

        Err(crate::error::LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
            "Dictionary '{}' has format version {}, but this build of Lindera reads format version {}. To fix this, {hint}.",
            self.name,
            self.format_version,
            DICTIONARY_FORMAT_VERSION,
        )))
    }

    /// Load metadata with fallback to default values.
    /// This is used when feature flags are disabled and data might be empty.
    pub fn load_or_default(data: &[u8], default_fn: fn() -> Self) -> Self {
        if data.is_empty() {
            default_fn()
        } else {
            match Self::load(data) {
                Ok(metadata) => metadata,
                Err(_) => default_fn(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_default() {
        let metadata = Metadata::default();
        assert_eq!(metadata.name, "default");
        // Schema no longer has name field
    }

    /// A source `metadata.json` -- the hand-written kind checked into each
    /// dictionary crate -- carries no `format_version` and must keep parsing.
    #[test]
    fn metadata_without_format_version_reads_as_legacy() {
        let json = serde_json::to_value(Metadata::default()).unwrap();
        let mut object = json.as_object().unwrap().clone();
        object.remove("format_version");
        let without = serde_json::to_vec(&object).unwrap();

        let metadata = Metadata::load(&without).unwrap();
        assert_eq!(metadata.format_version, LEGACY_FORMAT_VERSION);
    }

    /// v5.x shipped the format that is now version 1, so a dictionary built
    /// before the field existed must still load. If this fails after a bump of
    /// [`DICTIONARY_FORMAT_VERSION`], that is correct and expected -- the test
    /// documents the boundary rather than pinning it.
    #[test]
    fn legacy_version_is_the_first_format_version() {
        assert_eq!(LEGACY_FORMAT_VERSION, 1);
    }

    #[test]
    fn validate_format_version_accepts_the_current_version() {
        let metadata = Metadata::default();
        assert_eq!(metadata.format_version, DICTIONARY_FORMAT_VERSION);
        assert!(metadata.validate_format_version().is_ok());
    }

    #[test]
    fn validate_format_version_rejects_an_older_dictionary() {
        let metadata = Metadata {
            name: "ipadic".to_string(),
            format_version: DICTIONARY_FORMAT_VERSION - 1,
            ..Metadata::default()
        };

        let err = metadata.validate_format_version().unwrap_err().to_string();
        assert!(err.contains("ipadic"), "{err}");
        assert!(err.contains("lindera build"), "{err}");
    }

    #[test]
    fn validate_format_version_rejects_a_newer_dictionary() {
        let metadata = Metadata {
            format_version: DICTIONARY_FORMAT_VERSION + 1,
            ..Metadata::default()
        };

        let err = metadata.validate_format_version().unwrap_err().to_string();
        assert!(err.contains("upgrade Lindera"), "{err}");
    }

    /// The version must survive a JSON round trip; a `skip_serializing_if`
    /// added by accident would make every built dictionary claim to be legacy.
    #[test]
    fn format_version_round_trips_through_json() {
        let metadata = Metadata {
            format_version: 7,
            ..Metadata::default()
        };

        let json = serde_json::to_vec(&metadata).unwrap();
        assert_eq!(Metadata::load(&json).unwrap().format_version, 7);
    }

    #[test]
    fn test_metadata_new() {
        let schema = Schema::default();
        let metadata = Metadata::new(
            "TestDict".to_string(),
            "UTF-8".to_string(),
            -10000,
            0,
            0,
            "*".to_string(),
            false,
            false,
            false,
            schema.clone(),
            Schema::new(vec!["surface".to_string(), "reading".to_string()]),
        );
        assert_eq!(metadata.name, "TestDict");
        // Schema no longer has name field
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = Metadata::default();

        // Test serialization
        let serialized = serde_json::to_string(&metadata).unwrap();
        assert!(serialized.contains("default"));
        assert!(serialized.contains("schema"));
        assert!(serialized.contains("name"));

        // Test deserialization
        let deserialized: Metadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "default");
        // Schema no longer has name field
    }
}
