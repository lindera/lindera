use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::IPADICNEologdSchema;

/// IPADIC NEologd metadata factory
pub struct IPADICNEologdMetadata;

impl Default for IPADICNEologdMetadata {
    fn default() -> Self {
        Self
    }
}

impl IPADICNEologdMetadata {
    /// Create default IPADIC NEologd metadata
    pub fn metadata() -> Metadata {
        Metadata::new(
            "IPADIC-NEologd".to_string(),
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
            true,  // flexible_csv
            false, // skip_invalid_cost_or_id
            true,  // normalize_details is true for IPAdic-NEologd
            IPADICNEologdSchema::schema(),
            vec![
                Some(1), // Major POS classification
                None,    // Middle POS classification
                None,    // Small POS classification
                None,    // Fine POS classification
                None,    // Conjugation type
                None,    // Conjugation form
                Some(0), // Base form
                Some(2), // Reading
                None,    // Pronunciation
            ],
        )
    }
}
