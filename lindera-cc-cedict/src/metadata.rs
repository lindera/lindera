use lindera_dictionary::dictionary::metadata::Metadata;

pub struct CcCedictMetadata;

impl CcCedictMetadata {
    pub fn new() -> Metadata {
        Metadata::cc_cedict()
    }
}

impl Default for CcCedictMetadata {
    fn default() -> Self {
        Self
    }
}

impl From<CcCedictMetadata> for Metadata {
    fn from(_: CcCedictMetadata) -> Self {
        Metadata::cc_cedict()
    }
}