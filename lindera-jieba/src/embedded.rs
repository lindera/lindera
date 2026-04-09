#[cfg(feature = "embed-jieba")]
use std::env;

use lindera_dictionary::LinderaResult;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::loader::DictionaryLoader;

macro_rules! jieba_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-jieba")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-jieba"))]
        const $name: &'static [u8] = &[];
    };
}

// Metadata-specific macro (skips compression/decompression processing)
macro_rules! jieba_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-jieba")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-jieba"))]
        const $name: &'static [u8] = &[];
    };
}

jieba_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-jieba/char_def.bin",
    "char_def.bin"
);
jieba_data!(CONNECTION_DATA, "/lindera-jieba/matrix.mtx", "matrix.mtx");
jieba_data!(DA_DATA, "/lindera-jieba/dict.da", "dict.da");
jieba_data!(VALS_DATA, "/lindera-jieba/dict.vals", "dict.vals");
jieba_data!(UNKNOWN_DATA, "/lindera-jieba/unk.bin", "unk.bin");
jieba_data!(
    WORDS_IDX_DATA,
    "/lindera-jieba/dict.wordsidx",
    "dict.wordsidx"
);
jieba_data!(WORDS_DATA, "/lindera-jieba/dict.words", "dict.words");
jieba_metadata!(
    METADATA_DATA,
    "/lindera-jieba/metadata.json",
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

pub struct EmbeddedJiebaLoader;

impl Default for EmbeddedJiebaLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedJiebaLoader {
    pub fn new() -> Self {
        Self
    }
}

impl DictionaryLoader for EmbeddedJiebaLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }
}
