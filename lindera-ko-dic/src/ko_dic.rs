#[cfg(feature = "ko-dic")]
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

macro_rules! ko_dic_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "ko-dic")]
        decompress_data!(
            $name,
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path)),
            $filename
        );
        #[cfg(not(feature = "ko-dic"))]
        decompress_data!($name, &[], $filename);
    };
}

ko_dic_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-ko-dic/char_def.bin",
    "char_def.bin"
);
ko_dic_data!(CONNECTION_DATA, "/lindera-ko-dic/matrix.mtx", "matrix.mtx");
ko_dic_data!(DA_DATA, "/lindera-ko-dic/dict.da", "dict.da");
ko_dic_data!(VALS_DATA, "/lindera-ko-dic/dict.vals", "dict.vals");
ko_dic_data!(UNKNOWN_DATA, "/lindera-ko-dic/unk.bin", "unk.bin");
ko_dic_data!(
    WORDS_IDX_DATA,
    "/lindera-ko-dic/dict.wordsidx",
    "dict.wordsidx"
);
ko_dic_data!(WORDS_DATA, "/lindera-ko-dic/dict.words", "dict.words");

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
