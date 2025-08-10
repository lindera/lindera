#[cfg(feature = "embedded-ipadic")]
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

macro_rules! ipadic_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embedded-ipadic")]
        decompress_data!(
            $name,
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path)),
            $filename
        );
        #[cfg(not(feature = "embedded-ipadic"))]
        decompress_data!($name, &[], $filename);
    };
}

// Metadata-specific macro (skips compression/decompression processing)
macro_rules! ipadic_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embedded-ipadic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embedded-ipadic"))]
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
    // Load metadata from embedded binary data with fallback to default
    let metadata =
        Metadata::load_or_default(METADATA_DATA, crate::metadata::IPADICMetadata::metadata);

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
