#[cfg(feature = "embed-ipadic")]
use std::env;

use lindera_dictionary::LinderaResult;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::loader::DictionaryLoader;

macro_rules! ipadic_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-ipadic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-ipadic"))]
        const $name: &'static [u8] = &[];
    };
}

// Metadata-specific macro (skips compression/decompression processing)
macro_rules! ipadic_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-ipadic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-ipadic"))]
        const $name: &'static [u8] = &[];
    };
}

ipadic_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-ipadic/char_def.bin",
    "char_def.bin"
);
ipadic_data!(CONNECTION_DATA, "/lindera-ipadic/matrix.mtx", "matrix.mtx");
ipadic_data!(DA_DATA, "/lindera-ipadic/dict.da", "dict.da");
ipadic_data!(VALS_DATA, "/lindera-ipadic/dict.vals", "dict.vals");
ipadic_data!(UNKNOWN_DATA, "/lindera-ipadic/unk.bin", "unk.bin");
ipadic_data!(
    WORDS_IDX_DATA,
    "/lindera-ipadic/dict.wordsidx",
    "dict.wordsidx"
);
ipadic_data!(WORDS_DATA, "/lindera-ipadic/dict.words", "dict.words");
ipadic_metadata!(
    METADATA_DATA,
    "/lindera-ipadic/metadata.json",
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

pub struct EmbeddedIPADICLoader;

impl Default for EmbeddedIPADICLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedIPADICLoader {
    pub fn new() -> Self {
        Self
    }
}

impl DictionaryLoader for EmbeddedIPADICLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }
}
