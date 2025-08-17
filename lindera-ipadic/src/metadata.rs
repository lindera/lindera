use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::{IPADICDictionarySchema, IPADICUserDictionarySchema};
use crate::{DICTIONARY_ENCODING, DICTIONARY_NAME};

/// IPADIC metadata factory
pub struct IPADICMetadata;

impl Default for IPADICMetadata {
    fn default() -> Self {
        Self
    }
}

impl IPADICMetadata {
    /// Create default IPADIC metadata
    pub fn metadata() -> Metadata {
        Metadata::new(
            DICTIONARY_NAME.to_string(),
            DICTIONARY_ENCODING.to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            0,
            "*".to_string(),
            13,
            11,
            true,  // flexible_csv
            false, // skip_invalid_cost_or_id
            true,  // normalize_details is true for IPAdic
            IPADICDictionarySchema::schema(),
            IPADICUserDictionarySchema::schema(),
        )
    }
}
