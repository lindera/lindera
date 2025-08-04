use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::UnidicSchema;

/// UniDic metadata factory
pub struct UnidicMetadata;

impl UnidicMetadata {
    /// Create default UniDic metadata
    pub fn default() -> Metadata {
        Metadata::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            21,
            10,
            UnidicSchema::default(),
            "UniDic".to_string(),
        )
    }
}