use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::IpadicNeologdSchema;

/// IPADIC NEologd metadata factory
pub struct IpadicNeologdMetadata;

impl IpadicNeologdMetadata {
    /// Create default IPADIC NEologd metadata
    pub fn default() -> Metadata {
        Metadata::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
            IpadicNeologdSchema::default(),
            "IPADIC-NEologd".to_string(),
            false, // flexible_csv
            false, // skip_invalid_cost_or_id
            true,  // normalize_details is true for IPAdic-NEologd
            vec![
                Some(1),
                None,
                None,
                None,
                None,
                None,
                Some(0),
                Some(2),
                None,
            ],
        )
    }
}
