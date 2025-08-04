use lindera_dictionary::dictionary::metadata::Metadata;

pub struct IpadicNeologdMetadata;

impl IpadicNeologdMetadata {
    pub fn new() -> Metadata {
        Metadata::ipadic_neologd()
    }
}

impl Default for IpadicNeologdMetadata {
    fn default() -> Self {
        Self
    }
}

impl From<IpadicNeologdMetadata> for Metadata {
    fn from(_: IpadicNeologdMetadata) -> Self {
        Metadata::ipadic_neologd()
    }
}