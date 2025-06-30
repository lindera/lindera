#[cfg(feature = "cc-cedict")]
use std::env;
#[cfg(feature = "compress")]
use std::ops::Deref;

use lindera_dictionary::LinderaResult;
#[cfg(feature = "compress")]
use lindera_dictionary::decompress::decompress;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;

macro_rules! decompress_data {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "compress")]
        static $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            let (compressed_data, _) =
                bincode::serde::decode_from_slice(&$bytes[..], bincode::config::legacy())
                    .expect(concat!("invalid file format ", $filename));
            decompress(compressed_data).expect(concat!("invalid file format ", $filename))
        });
        #[cfg(not(feature = "compress"))]
        const $name: &'static [u8] = $bytes;
    };
}

macro_rules! cc_cedict_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "cc-cedict")]
        decompress_data!(
            $name,
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path)),
            $filename
        );
        #[cfg(not(feature = "cc-cedict"))]
        decompress_data!($name, &[], $filename);
    };
}

cc_cedict_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-cc-cedict/char_def.bin",
    "char_def.bin"
);
cc_cedict_data!(
    CONNECTION_DATA,
    "/lindera-cc-cedict/matrix.mtx",
    "matrix.mtx"
);
cc_cedict_data!(DA_DATA, "/lindera-cc-cedict/dict.da", "dict.da");
cc_cedict_data!(VALS_DATA, "/lindera-cc-cedict/dict.vals", "dict.vals");
cc_cedict_data!(UNKNOWN_DATA, "/lindera-cc-cedict/unk.bin", "unk.bin");
cc_cedict_data!(
    WORDS_IDX_DATA,
    "/lindera-cc-cedict/dict.wordsidx",
    "dict.wordsidx"
);
cc_cedict_data!(WORDS_DATA, "/lindera-cc-cedict/dict.words", "dict.words");

pub fn load() -> LinderaResult<Dictionary> {
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
        })
    }
}
