use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::{KoDicSchema, KoDicUserDictionarySchema};
use crate::{DICTIONARY_ENCODING, DICTIONARY_NAME};

/// Ko-Dic metadata factory
pub struct KoDicMetadata;

impl Default for KoDicMetadata {
    fn default() -> Self {
        Self
    }
}

impl KoDicMetadata {
    /// Create default Ko-Dic metadata
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
            12,
            12,
            false, // flexible_csv
            false, // skip_invalid_cost_or_id
            false, // normalize_details
            KoDicSchema::schema(),
            KoDicUserDictionarySchema::schema(),
        )
    }
}
