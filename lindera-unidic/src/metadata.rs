use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::UniDicSchema;
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
            3,
            -10000,
            0,
            0,
            "*".to_string(),
            21,
            10,
            false, // flexible_csv
            false, // skip_invalid_cost_or_id
            false, // normalize_details
            UniDicSchema::schema(),
            vec![
                Some(1), // Part-of-speech
                None,    // Part-of-speech subcategory 1
                None,    // Part-of-speech subcategory 2
                None,    // Part-of-speech subcategory 3
                None,    // Conjugation form
                None,    // Conjugation type
                Some(2), // Reading
                None,    // Lexeme
                None,    // Orthographic surface form
                None,    // Phonological surface form
                None,    // Orthographic base form
                None,    // Phonological base form
                None,    // Word type
                None,    // Initial mutation type
                None,    // Initial mutation form
                None,    // Final mutation type
                None,    // Final mutation form
            ],
        )
    }
}
