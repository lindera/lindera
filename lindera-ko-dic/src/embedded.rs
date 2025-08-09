#[cfg(feature = "embedded-ko-dic")]
use std::env;
#[cfg(feature = "compress")]
use std::ops::Deref;

use lindera_dictionary::LinderaResult;
#[cfg(feature = "compress")]
use lindera_dictionary::decompress::{CompressedData, decompress};
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::dictionary_loader::DictionaryLoader;

macro_rules! decompress_data {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "compress")]
        static $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            // First check if this is compressed data by attempting to decode as CompressedData
            match bincode::serde::decode_from_slice::<CompressedData, _>(
                &$bytes[..],
                bincode::config::legacy(),
            ) {
                Ok((compressed_data, _)) => {
                    // Successfully decoded as CompressedData, now decompress it
                    match decompress(compressed_data) {
                        Ok(decompressed) => decompressed,
                        Err(_) => {
                            // Decompression failed, fall back to raw data
                            $bytes.to_vec()
                        }
                    }
                }
                Err(_) => {
                    // Not compressed data format, use as raw binary
                    $bytes.to_vec()
                }
            }
        });
        #[cfg(not(feature = "compress"))]
        const $name: &'static [u8] = $bytes;
    };
}

macro_rules! kodic_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embedded-ko-dic")]
        decompress_data!(
            $name,
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path)),
            $filename
        );
        #[cfg(not(feature = "embedded-ko-dic"))]
        decompress_data!($name, &[], $filename);
    };
}

// Metadata-specific macro (skips compression/decompression processing)
macro_rules! kodic_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embedded-ko-dic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embedded-ko-dic"))]
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
    // Load metadata from embedded binary data with fallback to default
    let metadata =
        Metadata::load_or_default(METADATA_DATA, crate::metadata::KoDicMetadata::metadata);

    #[cfg(feature = "compress")]
    {
        Ok(Dictionary {
            prefix_dictionary: PrefixDictionary::load(
                DA_DATA.deref(),
                VALS_DATA.deref(),
                WORDS_IDX_DATA.deref(),
                WORDS_DATA.deref(),
                true,
            ),
            connection_cost_matrix: ConnectionCostMatrix::load(CONNECTION_DATA.deref()),
            character_definition: CharacterDefinition::load(&CHAR_DEFINITION_DATA)?,
            unknown_dictionary: UnknownDictionary::load(&UNKNOWN_DATA)?,
            metadata,
        })
    }
    #[cfg(not(feature = "compress"))]
    {
        Ok(Dictionary {
            prefix_dictionary: PrefixDictionary::load(
                DA_DATA,
                VALS_DATA,
                WORDS_IDX_DATA,
                WORDS_DATA,
                true,
            ),
            connection_cost_matrix: ConnectionCostMatrix::load(CONNECTION_DATA),
            character_definition: CharacterDefinition::load(CHAR_DEFINITION_DATA)?,
            unknown_dictionary: UnknownDictionary::load(UNKNOWN_DATA)?,
            metadata,
        })
    }
}

pub struct EmbeddedLoader;

impl EmbeddedLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }
}

impl DictionaryLoader for EmbeddedLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }
}
