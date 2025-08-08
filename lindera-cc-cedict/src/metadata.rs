use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::CcCedictSchema;

/// CC-CEDICT metadata factory
pub struct CcCedictMetadata;

impl CcCedictMetadata {
    /// Create default CC-CEDICT metadata
    pub fn default() -> Metadata {
        Metadata::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            12,
            10,
            CcCedictSchema::default(),
            "CC-CEDICT".to_string(),
            true,  // flexible_csv is true for CC-CEDICT
            true,  // skip_invalid_cost_or_id is true for CC-CEDICT
            false, // normalize_details
            vec![
                Some(1), // Major POS classification
                None,    // Middle POS classification
                None,    // Small POS classification
                None,    // Fine POS classification
                Some(2), // Pinyin
                None,    // Traditional
                None,    // Simplified
                None,    // definition
            ],
        )
    }
}
