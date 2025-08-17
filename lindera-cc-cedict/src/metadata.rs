use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::{CCedictUserDictionarySchema, CcCedictDictionarySchema};
use crate::{DICTIONARY_ENCODING, DICTIONARY_NAME};

/// CC-CEDICT metadata factory
pub struct CcCedictMetadata;

impl Default for CcCedictMetadata {
    fn default() -> Self {
        Self
    }
}

impl CcCedictMetadata {
    /// Create default CC-CEDICT metadata
    pub fn metadata() -> Metadata {
        Metadata::new(
            DICTIONARY_NAME.to_string(),
            DICTIONARY_ENCODING.to_string(),
            Algorithm::Deflate,
            -10000,
            0,
            0,
            "*".to_string(),
            10,
            true,  // flexible_csv is true for CC-CEDICT
            true,  // skip_invalid_cost_or_id is true for CC-CEDICT
            false, // normalize_details
            CcCedictDictionarySchema::schema(),
            CCedictUserDictionarySchema::schema(),
        )
    }
}
