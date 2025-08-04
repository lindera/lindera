use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::IpadicSchema;

/// IPADIC metadata factory
pub struct IpadicMetadata;

impl IpadicMetadata {
    /// Create default IPADIC metadata
    pub fn default() -> Metadata {
        Metadata::new(
            "EUC-JP".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
            IpadicSchema::default(),
            "IPADIC".to_string(),
        )
    }
}