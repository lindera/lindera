use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::IpadicSchema;

/// IPADIC metadata factory
pub struct IpadicMetadata;

impl IpadicMetadata {
    /// Create default IPADIC metadata
    pub fn default() -> Metadata {
        Metadata::new(
            "IPADIC".to_string(),
            "EUC-JP".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
            false, // flexible_csv
            false, // skip_invalid_cost_or_id
            true,  // normalize_details is true for IPAdic
            IpadicSchema::default(),
            vec![
                Some(1), // Major POS classification
                None,    // Middle POS classification
                None,    // Small POS classification
                None,    // Fine POS classification
                None,    // Conjugation type
                None,    // Conjugation form
                None,    // Base form
                Some(2), // Reading
                None,    // Pronunciation
            ],
        )
    }
}
