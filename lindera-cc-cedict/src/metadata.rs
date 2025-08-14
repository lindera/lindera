use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::CcCedictSchema;
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
            3,
            -10000,
            0,
            12,
            10,
            true,  // flexible_csv is true for CC-CEDICT
            true,  // skip_invalid_cost_or_id is true for CC-CEDICT
            false, // normalize_details
            CcCedictSchema::schema(),
            vec![
                Some(1), // Part-of-speech
                None,    // Part-of-speech subcategory 1
                None,    // Part-of-speech subcategory 2
                None,    // Part-of-speech subcategory 3
                Some(2), // Pinyin
                None,    // Traditional
                None,    // Simplified
                None,    // Definition
            ],
        )
    }
}
