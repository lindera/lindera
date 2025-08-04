use lindera_dictionary::dictionary::metadata::Metadata;

pub struct KoDicMetadata;

impl KoDicMetadata {
    pub fn new() -> Metadata {
        Metadata::ko_dic()
    }
}

impl Default for KoDicMetadata {
    fn default() -> Self {
        Self
    }
}

impl From<KoDicMetadata> for Metadata {
    fn from(_: KoDicMetadata) -> Self {
        Metadata::ko_dic()
    }
}