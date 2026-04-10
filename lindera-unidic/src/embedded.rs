#[cfg(feature = "embed-unidic")]
use std::env;

use lindera_dictionary::LinderaResult;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::loader::DictionaryLoader;

macro_rules! unidic_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-unidic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-unidic"))]
        const $name: &'static [u8] = &[];
    };
}

// Metadata-specific macro
macro_rules! unidic_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-unidic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-unidic"))]
        const $name: &'static [u8] = &[];
    };
}

unidic_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-unidic/char_def.bin",
    "char_def.bin"
);
unidic_data!(CONNECTION_DATA, "/lindera-unidic/matrix.mtx", "matrix.mtx");
unidic_data!(DA_DATA, "/lindera-unidic/dict.da", "dict.da");
unidic_data!(VALS_DATA, "/lindera-unidic/dict.vals", "dict.vals");
unidic_data!(UNKNOWN_DATA, "/lindera-unidic/unk.bin", "unk.bin");
unidic_data!(
    WORDS_IDX_DATA,
    "/lindera-unidic/dict.wordsidx",
    "dict.wordsidx"
);
unidic_data!(WORDS_DATA, "/lindera-unidic/dict.words", "dict.words");
unidic_metadata!(
    METADATA_DATA,
    "/lindera-unidic/metadata.json",
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

pub struct EmbeddedUniDicLoader;

impl Default for EmbeddedUniDicLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedUniDicLoader {
    pub fn new() -> Self {
        Self
    }
}

impl DictionaryLoader for EmbeddedUniDicLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }
}
