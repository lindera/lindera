use serde::{Deserialize, Serialize};

use crate::decompress::Algorithm;
use crate::dictionary::schema::Schema;

const DEFAULT_COMPRESS_ALGORITHM: Algorithm = Algorithm::Deflate;
const DEFAULT_WORD_COST: i16 = -10000;
const DEFAULT_LEFT_CONTEXT_ID: u16 = 1288;
const DEFAULT_RIGHT_CONTEXT_ID: u16 = 1288;
const DEFAULT_FIELD_VALUE: &str = "*";

#[derive(Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,                              // Name of the dictionary
    pub encoding: String,                          // Character encoding
    pub compress_algorithm: Algorithm,             // Compression algorithm
    pub user_dictionary_fields_num: usize,         // Number of fields in simple user dictionary
    pub default_word_cost: i16,                    // Word cost for simple user dictionary
    pub default_left_context_id: u16,              // Context ID for simple user dictionary
    pub default_right_context_id: u16,             // Context ID for simple user dictionary
    pub default_field_value: String, // Default value for fields in simple user dictionary
    pub dictionary_fields_num: usize, // Number of fields in detailed user dictionary
    pub unk_fields_num: usize,       // Number of fields in unknown dictionary
    pub flexible_csv: bool,          // Handle CSV columns flexibly
    pub skip_invalid_cost_or_id: bool, // Skip invalid cost or ID
    pub normalize_details: bool,     // Normalize characters
    pub schema: Schema,              // Schema for the dictionary
    pub userdic_field_indices: Vec<Option<usize>>, // User dictionary field indices
}

impl Default for Metadata {
    fn default() -> Self {
        // Default metadata values can be adjusted as needed
        Metadata::new(
            "default".to_string(),
            "UTF-8".to_string(),
            DEFAULT_COMPRESS_ALGORITHM,
            3,
            DEFAULT_WORD_COST,
            DEFAULT_LEFT_CONTEXT_ID,
            DEFAULT_RIGHT_CONTEXT_ID,
            DEFAULT_FIELD_VALUE.to_string(),
            13,
            11,
            false,
            false,
            false,
            Schema::default(),
            vec![
                Some(1), // Use field 1 for major POS classification
                None,    // Middle POS classification is '*'
                None,    // Small POS classification is '*'
                None,    // Fine POS classification is '*'
                None,    // Conjugation type is '*'
                None,    // Conjugation form is '*'
                Some(0), // Use field 0 for base form
                Some(2), // Use field 2 for reading
                None,    // Pronunciation is '*'
            ],
        )
    }
}

impl Metadata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        encoding: String,
        compress_algorithm: Algorithm,
        simple_userdic_fields_num: usize,
        simple_word_cost: i16,
        default_left_context_id: u16,
        default_right_context_id: u16,
        default_field_value: String,
        detailed_userdic_fields_num: usize,
        unk_fields_num: usize,
        flexible_csv: bool,
        skip_invalid_cost_or_id: bool,
        normalize_details: bool,
        schema: Schema,
        userdic_field_indices: Vec<Option<usize>>,
    ) -> Self {
        Self {
            encoding,
            compress_algorithm,
            user_dictionary_fields_num: simple_userdic_fields_num,
            default_word_cost: simple_word_cost,
            default_left_context_id,
            default_right_context_id,
            default_field_value,
            dictionary_fields_num: detailed_userdic_fields_num,
            unk_fields_num,
            schema,
            name,
            flexible_csv,
            skip_invalid_cost_or_id,
            normalize_details,
            userdic_field_indices,
        }
    }

    /// Load metadata from binary data (JSON format or compressed binary format).
    /// This provides a consistent interface with other dictionary components.
    pub fn load(data: &[u8]) -> crate::LinderaResult<Self> {
        // If data is empty, return an error since metadata is required
        if data.is_empty() {
            return Err(crate::error::LinderaErrorKind::Io
                .with_error(anyhow::anyhow!("Empty metadata data")));
        }

        // Try to deserialize as JSON first (for uncompressed metadata.json files)
        if let Ok(metadata) = serde_json::from_slice(data) {
            return Ok(metadata);
        }

        // If JSON fails, try to decompress as bincode-encoded compressed data
        #[cfg(feature = "compress")]
        {
            use crate::decompress::{CompressedData, decompress};

            if let Ok((compressed_data, _)) = bincode::serde::decode_from_slice::<CompressedData, _>(
                data,
                bincode::config::legacy(),
            ) {
                if let Ok(decompressed) = decompress(compressed_data) {
                    // Try to parse the decompressed data as JSON
                    if let Ok(metadata) = serde_json::from_slice(&decompressed) {
                        return Ok(metadata);
                    }
                }
            }
        }

        #[cfg(not(feature = "compress"))]
        {
            // Without compress feature, data should be raw JSON
            return serde_json::from_slice(data).map_err(|err| {
                crate::error::LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err))
            });
        }

        // If all attempts fail, return an error
        Err(
            crate::error::LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "Failed to deserialize metadata from any supported format"
            )),
        )
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

    #[test]
    fn test_metadata_new() {
        let schema = Schema::default();
        let metadata = Metadata::new(
            "TestDict".to_string(),
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            0,
            "*".to_string(),
            21,
            10,
            false,
            false,
            false,
            schema.clone(),
            vec![Some(1), None, None, None, None, None, Some(2), None],
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
