#[cfg(feature = "embed-ko-dic")]
use std::env;

use lindera_dictionary::LinderaResult;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::loader::DictionaryLoader;

macro_rules! kodic_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-ko-dic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-ko-dic"))]
        const $name: &'static [u8] = &[];
    };
}

// Metadata-specific macro
macro_rules! kodic_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-ko-dic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-ko-dic"))]
        const $name: &'static [u8] = &[];
    };
}

kodic_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-ko-dic/char_def.bin",
    "char_def.bin"
);
kodic_data!(CONNECTION_DATA, "/lindera-ko-dic/matrix.mtx", "matrix.mtx");
kodic_data!(DA_DATA, "/lindera-ko-dic/dict.da", "dict.da");
kodic_data!(VALS_DATA, "/lindera-ko-dic/dict.vals", "dict.vals");
kodic_data!(UNKNOWN_DATA, "/lindera-ko-dic/unk.bin", "unk.bin");
kodic_data!(
    WORDS_IDX_DATA,
    "/lindera-ko-dic/dict.wordsidx",
    "dict.wordsidx"
);
kodic_data!(WORDS_DATA, "/lindera-ko-dic/dict.words", "dict.words");
kodic_metadata!(
    METADATA_DATA,
    "/lindera-ko-dic/metadata.json",
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

pub struct EmbeddedKoDicLoader;

impl Default for EmbeddedKoDicLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedKoDicLoader {
    pub fn new() -> Self {
        Self
    }
}

impl DictionaryLoader for EmbeddedKoDicLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }
}
