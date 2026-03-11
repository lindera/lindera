#[cfg(feature = "embed-jieba")]
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
use lindera_dictionary::loader::DictionaryLoader;

macro_rules! decompress_data {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "compress")]
        static $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            // First check if this is compressed data by attempting to check aligned root
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&$bytes[..]);
            match rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned) {
                Ok(compressed_data) => {
                    // Decompress it
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

macro_rules! jieba_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-jieba")]
        decompress_data!(
            $name,
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path)),
            $filename
        );
        #[cfg(not(feature = "embed-jieba"))]
        decompress_data!($name, &[], $filename);
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
    // Load metadata from embedded binary data
    let metadata = Metadata::load(METADATA_DATA)?;

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
