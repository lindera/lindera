use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::IPADICSchema;
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
            13,
            11,
            true,  // flexible_csv
            false, // skip_invalid_cost_or_id
            true,  // normalize_details is true for IPAdic
            IPADICSchema::schema(),
            vec![
                Some(1), // Part-of-speech
                None,    // Part-of-speech subcategory 1
                None,    // Part-of-speech subcategory 2
                None,    // Part-of-speech subcategory 3
                None,    // Conjugation type
                None,    // Conjugation form
                Some(0), // Base form
                Some(2), // Reading
                None,    // Pronunciation
            ],
        )
    }
}
