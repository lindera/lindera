use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::KoDicSchema;

/// Ko-Dic metadata factory
pub struct KoDicMetadata;

impl KoDicMetadata {
    /// Create default Ko-Dic metadata
    pub fn default() -> Metadata {
        Metadata::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            12,
            12,
            KoDicSchema::default(),
            "KO-DIC".to_string(),
            false,              // flexible_csv
            false,              // skip_invalid_cost_or_id
            false,              // normalize_details
        )
    }
}
