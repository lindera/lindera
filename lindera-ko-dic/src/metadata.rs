use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::KoDicSchema;
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
            12,
            12,
            false, // flexible_csv
            false, // skip_invalid_cost_or_id
            false, // normalize_details
            KoDicSchema::schema(),
            vec![
                Some(1), // Part-of-speech tag
                None,    // Meaning
                None,    // Presence or absence
                Some(2), // Reading
                None,    // Type
                None,    // First part-of-speech
                None,    // Last part-of-speech
                None,    // Expression
            ],
        )
    }
}
