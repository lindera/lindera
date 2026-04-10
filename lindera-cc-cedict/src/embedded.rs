#[cfg(feature = "embed-cc-cedict")]
use std::env;

use lindera_dictionary::LinderaResult;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::loader::DictionaryLoader;

macro_rules! cccedict_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-cc-cedict")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-cc-cedict"))]
        const $name: &'static [u8] = &[];
    };
}

// Metadata-specific macro
macro_rules! cccedict_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-cc-cedict")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-cc-cedict"))]
        const $name: &'static [u8] = &[];
    };
}

cccedict_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-cc-cedict/char_def.bin",
    "char_def.bin"
);
cccedict_data!(
    CONNECTION_DATA,
    "/lindera-cc-cedict/matrix.mtx",
    "matrix.mtx"
);
cccedict_data!(DA_DATA, "/lindera-cc-cedict/dict.da", "dict.da");
cccedict_data!(VALS_DATA, "/lindera-cc-cedict/dict.vals", "dict.vals");
cccedict_data!(UNKNOWN_DATA, "/lindera-cc-cedict/unk.bin", "unk.bin");
cccedict_data!(
    WORDS_IDX_DATA,
    "/lindera-cc-cedict/dict.wordsidx",
    "dict.wordsidx"
);
cccedict_data!(WORDS_DATA, "/lindera-cc-cedict/dict.words", "dict.words");
cccedict_metadata!(
    METADATA_DATA,
    "/lindera-cc-cedict/metadata.json",
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

pub struct EmbeddedCcCedictLoader;

impl Default for EmbeddedCcCedictLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedCcCedictLoader {
    pub fn new() -> Self {
        Self
    }
}

impl DictionaryLoader for EmbeddedCcCedictLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }
}
