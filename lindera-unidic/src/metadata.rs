use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::{UniDicSchema, UniDicUserDictionarySchema};
use crate::{DICTIONARY_ENCODING, DICTIONARY_NAME};

/// UniDic metadata factory
pub struct UniDicMetadata;

impl Default for UniDicMetadata {
    fn default() -> Self {
        Self
    }
}

impl UniDicMetadata {
    /// Create default UniDic metadata
    pub fn metadata() -> Metadata {
        Metadata::new(
            DICTIONARY_NAME.to_string(),
            DICTIONARY_ENCODING.to_string(),
            Algorithm::Deflate,
            -10000,
            0,
            0,
            "*".to_string(),
            false, // flexible_csv
            false, // skip_invalid_cost_or_id
            false, // normalize_details
            UniDicSchema::schema(),
            UniDicUserDictionarySchema::schema(),
        )
    }
}
