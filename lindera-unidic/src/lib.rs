#[cfg(feature = "unidic")]
use std::env;

use lindera_core::character_definition::CharacterDefinitions;
use lindera_core::connection::ConnectionCostMatrix;
use lindera_core::dictionary::Dictionary;
use lindera_core::prefix_dict::PrefixDict;
use lindera_core::unknown_dictionary::UnknownDictionary;
use lindera_core::LinderaResult;
#[cfg(feature = "compress")]
use lindera_decompress::decompress;

macro_rules! decompress_data {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "compress")]
        const $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            let compressed_data = bincode::deserialize_from(&$bytes[..])
                .expect(concat!("invalid file format ", $filename));
            decompress(compressed_data).expect(concat!("invalid file format ", $filename))
        });
        #[cfg(not(feature = "compress"))]
        const $name: &'static [u8] = $bytes;
    };
}

#[cfg(feature = "unidic")]
decompress_data!(
    CHAR_DEFINITION_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-unidic/char_def.bin")),
    "char_def.bin"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(CHAR_DEFINITION_DATA, &[], "char_def.bin");

#[cfg(feature = "unidic")]
decompress_data!(
    CONNECTION_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-unidic/matrix.mtx")),
    "matrix.mtx"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(CONNECTION_DATA, &[], "matrix.mtx");

#[cfg(feature = "unidic")]
decompress_data!(
    UNIDIC_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-unidic/dict.da")),
    "dict.da"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(UNIDIC_DATA, &[], "dict.da");

#[cfg(feature = "unidic")]
decompress_data!(
    UNIDIC_VALS,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-unidic/dict.vals")),
    "dict.vals"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(UNIDIC_VALS, &[], "dict.vals");

#[cfg(feature = "unidic")]
decompress_data!(
    UNKNOWN_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-unidic/unk.bin")),
    "unk.bin"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(UNKNOWN_DATA, &[], "unk.bin");

#[cfg(feature = "unidic")]
decompress_data!(
    WORDS_IDX_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-unidic/dict.wordsidx")),
    "dict.wordsidx"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(WORDS_IDX_DATA, &[], "dict.wordsidx");

#[cfg(feature = "unidic")]
decompress_data!(
    WORDS_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-unidic/dict.words")),
    "dict.words"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(WORDS_DATA, &[], "dict.words");

pub fn load_dictionary() -> LinderaResult<Dictionary> {
    Ok(Dictionary {
        dict: prefix_dict(),
        cost_matrix: connection(),
        char_definitions: char_def()?,
        unknown_dictionary: unknown_dict()?,
        words_idx_data: words_idx_data(),
        words_data: words_data(),
    })
}

pub fn char_def() -> LinderaResult<CharacterDefinitions> {
    CharacterDefinitions::load(&CHAR_DEFINITION_DATA)
}

pub fn connection() -> ConnectionCostMatrix {
    ConnectionCostMatrix::load(&CONNECTION_DATA)
}

pub fn prefix_dict() -> PrefixDict {
    PrefixDict::from_static_slice(&UNIDIC_DATA, &UNIDIC_VALS)
}

pub fn unknown_dict() -> LinderaResult<UnknownDictionary> {
    UnknownDictionary::load(&UNKNOWN_DATA)
}

pub fn words_idx_data() -> Vec<u8> {
    WORDS_IDX_DATA.to_vec()
}

pub fn words_data() -> Vec<u8> {
    WORDS_DATA.to_vec()
}
