#[cfg(feature = "embed-ipadic-neologd")]
use std::env;

use lindera_dictionary::LinderaResult;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::loader::DictionaryLoader;

macro_rules! ipadicneologd_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-ipadic-neologd")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-ipadic-neologd"))]
        const $name: &'static [u8] = &[];
    };
}

// Metadata-specific macro
macro_rules! ipadicneologd_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-ipadic-neologd")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-ipadic-neologd"))]
        const $name: &'static [u8] = &[];
    };
}

ipadicneologd_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-ipadic-neologd/char_def.bin",
    "char_def.bin"
);
ipadicneologd_data!(
    CONNECTION_DATA,
    "/lindera-ipadic-neologd/matrix.mtx",
    "matrix.mtx"
);
ipadicneologd_data!(DA_DATA, "/lindera-ipadic-neologd/dict.da", "dict.da");
ipadicneologd_data!(VALS_DATA, "/lindera-ipadic-neologd/dict.vals", "dict.vals");
ipadicneologd_data!(UNKNOWN_DATA, "/lindera-ipadic-neologd/unk.bin", "unk.bin");
ipadicneologd_data!(
    WORDS_IDX_DATA,
    "/lindera-ipadic-neologd/dict.wordsidx",
    "dict.wordsidx"
);
ipadicneologd_data!(
    WORDS_DATA,
    "/lindera-ipadic-neologd/dict.words",
    "dict.words"
);
ipadicneologd_metadata!(
    METADATA_DATA,
    "/lindera-ipadic-neologd/metadata.json",
    "metadata.json"
);

pub fn load() -> LinderaResult<Dictionary> {
    let metadata = Metadata::load(METADATA_DATA)?;
    let prefix_dictionary =
        PrefixDictionary::load(DA_DATA, VALS_DATA, WORDS_IDX_DATA, WORDS_DATA, true)?;
    let connection_cost_matrix = ConnectionCostMatrix::load(CONNECTION_DATA)?;
    let character_definition = CharacterDefinition::load(CHAR_DEFINITION_DATA)?;
    let unknown_dictionary = UnknownDictionary::load(UNKNOWN_DATA)?;

    Ok(Dictionary {
        prefix_dictionary,
        connection_cost_matrix,
        character_definition,
        unknown_dictionary,
        metadata,
    })
}

pub struct EmbeddedIPADICNEologdLoader;

impl Default for EmbeddedIPADICNEologdLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedIPADICNEologdLoader {
    pub fn new() -> Self {
        Self
    }
}

impl DictionaryLoader for EmbeddedIPADICNEologdLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }
}
