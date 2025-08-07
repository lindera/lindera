use serde::{Deserialize, Serialize};

use crate::decompress::Algorithm;
use crate::dictionary::schema::Schema;

#[derive(Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub encoding: String,
    pub compress_algorithm: Algorithm,
    pub simple_userdic_fields_num: usize,
    pub simple_word_cost: i16,
    pub simple_context_id: u16,
    pub detailed_userdic_fields_num: usize,
    pub unk_fields_num: usize,
    pub schema: Schema,
    pub name: String,
    pub flexible_csv: bool,
    pub skip_invalid_cost_or_id: bool,
    pub normalize_details: bool,
}

impl Default for Metadata {
    fn default() -> Self {
        // Default metadata values can be adjusted as needed
        Metadata::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
            Schema::ipadic(),
            "IPADIC".to_string(),
            false,
            false,
            false,
        )
    }
}

impl Metadata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        encoding: String,
        compress_algorithm: Algorithm,
        simple_userdic_fields_num: usize,
        simple_word_cost: i16,
        simple_context_id: u16,
        detailed_userdic_fields_num: usize,
        unk_fields_num: usize,
        schema: Schema,
        name: String,
        flexible_csv: bool,
        skip_invalid_cost_or_id: bool,
        normalize_details: bool,
    ) -> Self {
        Self {
            encoding,
            compress_algorithm,
            simple_userdic_fields_num,
            simple_word_cost,
            simple_context_id,
            detailed_userdic_fields_num,
            unk_fields_num,
            schema,
            name,
            flexible_csv,
            skip_invalid_cost_or_id,
            normalize_details,
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
        assert_eq!(metadata.name, "IPADIC");
        assert_eq!(metadata.schema.name, "IPADIC");
    }

    #[test]
    fn test_metadata_new() {
        let schema = Schema::unidic();
        let metadata = Metadata::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            21,
            10,
            schema.clone(),
            "TestDict".to_string(),
            false,
            false,
            false,
        );
        assert_eq!(metadata.name, "TestDict");
        assert_eq!(metadata.schema.name, schema.name);
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = Metadata::default();

        // Test serialization
        let serialized = serde_json::to_string(&metadata).unwrap();
        assert!(serialized.contains("IPADIC"));
        assert!(serialized.contains("schema"));
        assert!(serialized.contains("name"));

        // Test deserialization
        let deserialized: Metadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "IPADIC");
        assert_eq!(deserialized.schema.name, "IPADIC");
    }
}
