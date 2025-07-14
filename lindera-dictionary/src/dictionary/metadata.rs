use serde::{Deserialize, Serialize};

use crate::decompress::Algorithm;

#[derive(Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub encoding: String,
    pub compress_algorithm: Algorithm,
    pub simple_userdic_fields_num: usize,
    pub simple_word_cost: i16,
    pub simple_context_id: u16,
    pub detailed_userdic_fields_num: usize,
    pub unk_fields_num: usize,
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
        )
    }
}

impl Metadata {
    pub fn new(
        encoding: String,
        compress_algorithm: Algorithm,
        simple_userdic_fields_num: usize,
        simple_word_cost: i16,
        simple_context_id: u16,
        detailed_userdic_fields_num: usize,
        unk_fields_num: usize,
    ) -> Self {
        Self {
            encoding,
            compress_algorithm,
            simple_userdic_fields_num,
            simple_word_cost,
            simple_context_id,
            detailed_userdic_fields_num,
            unk_fields_num,
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

    pub fn ipadic() -> Self {
        Self::new(
            "EUC-JP".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
        )
    }

    pub fn ipadic_neologd() -> Self {
        Self::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
        )
    }

    pub fn unidic() -> Self {
        Self::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            21,
            10,
        )
    }

    pub fn ko_dic() -> Self {
        Self::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            12,
            12,
        )
    }

    pub fn cc_cedict() -> Self {
        Self::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            12,
            10,
        )
    }
}
