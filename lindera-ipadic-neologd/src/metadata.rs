use lindera_dictionary::decompress::Algorithm;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::schema::{IPADICNEologdSchema, IPADICNEologdUserDictionarySchema};
use crate::{DICTIONARY_ENCODING, DICTIONARY_NAME};

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
            DICTIONARY_NAME.to_string(),
            DICTIONARY_ENCODING.to_string(),
            Algorithm::Deflate,
            -10000,
            0,
            0,
            "*".to_string(),
            true,  // flexible_csv
            false, // skip_invalid_cost_or_id
            true,  // normalize_details is true for IPAdic-NEologd
            IPADICNEologdSchema::schema(),
            IPADICNEologdUserDictionarySchema::schema(),
        )
    }
}
