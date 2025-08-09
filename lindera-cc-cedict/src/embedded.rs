#[cfg(feature = "embedded-cc-cedict")]
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

macro_rules! cccedict_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embedded-cc-cedict")]
        decompress_data!(
            $name,
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path)),
            $filename
        );
        #[cfg(not(feature = "embedded-cc-cedict"))]
        decompress_data!($name, &[], $filename);
    };
}

// Metadata-specific macro (skips compression/decompression processing)
macro_rules! cccedict_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embedded-cc-cedict")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embedded-cc-cedict"))]
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
    // Load metadata from embedded binary data with fallback to default
    let metadata = Metadata::load_or_default(METADATA_DATA, || {
        crate::metadata::CcCedictMetadata::metadata()
    });

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
