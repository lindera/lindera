use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::UnidicSchema;

/// UniDic metadata factory
pub struct UnidicMetadata;

impl UnidicMetadata {
    /// Create default UniDic metadata
    pub fn default() -> Metadata {
        Metadata::new(
            "UniDic".to_string(),
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            21,
            10,
            false, // flexible_csv
            false, // skip_invalid_cost_or_id
            false, // normalize_details
            UnidicSchema::default(),
            vec![
                Some(1), // Major POS classification
                None,    // Middle POS classification
                None,    // Small POS classification
                None,    // Fine POS classification
                None,    // Conjugation form
                None,    // Conjugation type
                Some(2), // Lexeme reading
                None,    // Lexeme
                None,    // Orthography appearance type
                None,    // Pronunciation appearance type
                None,    // Orthography basic type
                None,    // Pronunciation basic type
                None,    // Word type
                None,    // Prefix of a word form
                None,    // Prefix of a word type
                None,    // Suffix of a word form
                None,    // Suffix of a word type
            ],
        )
    }
}
