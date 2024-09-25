#[cfg(feature = "cc-cedict")]
use std::env;

#[cfg(feature = "compress")]
use lindera_core::decompress::decompress;
use lindera_core::dictionary::character_definition::CharacterDefinition;
use lindera_core::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_core::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_core::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_core::dictionary::Dictionary;
use lindera_core::LinderaResult;

macro_rules! decompress_data {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "compress")]
        static $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            let compressed_data = bincode::deserialize_from(&$bytes[..])
                .expect(concat!("invalid file format ", $filename));
            decompress(compressed_data).expect(concat!("invalid file format ", $filename))
        });
        #[cfg(not(feature = "compress"))]
        const $name: &'static [u8] = $bytes;
    };
}

#[cfg(feature = "cc-cedict")]
decompress_data!(
    CHAR_DEFINITION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-cc-cedict/char_def.bin"
    )),
    "char_def.bin"
);
#[cfg(not(feature = "cc-cedict"))]
decompress_data!(CHAR_DEFINITION_DATA, &[], "char_def.bin");

#[cfg(feature = "cc-cedict")]
decompress_data!(
    CONNECTION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-cc-cedict/matrix.mtx"
    )),
    "matrix.mtx"
);
#[cfg(not(feature = "cc-cedict"))]
decompress_data!(CONNECTION_DATA, &[], "matrix.mtx");

#[cfg(feature = "cc-cedict")]
decompress_data!(
    CC_CEDICT_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-cc-cedict/dict.da"
    )),
    "dict.da"
);
#[cfg(not(feature = "cc-cedict"))]
decompress_data!(CC_CEDICT_DATA, &[], "dict.da");

#[cfg(feature = "cc-cedict")]
decompress_data!(
    CC_CEDICT_VALS,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-cc-cedict/dict.vals"
    )),
    "dict.vals"
);
#[cfg(not(feature = "cc-cedict"))]
decompress_data!(CC_CEDICT_VALS, &[], "dict.vals");

#[cfg(feature = "cc-cedict")]
decompress_data!(
    UNKNOWN_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-cc-cedict/unk.bin"
    )),
    "unk.bin"
);
#[cfg(not(feature = "cc-cedict"))]
decompress_data!(UNKNOWN_DATA, &[], "unk.bin");

#[cfg(feature = "cc-cedict")]
decompress_data!(
    WORDS_IDX_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-cc-cedict/dict.wordsidx"
    )),
    "dict.wordsidx"
);
#[cfg(not(feature = "cc-cedict"))]
decompress_data!(WORDS_IDX_DATA, &[], "dict.wordsidx");

#[cfg(feature = "cc-cedict")]
decompress_data!(
    WORDS_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-cc-cedict/dict.words"
    )),
    "dict.words"
);
#[cfg(not(feature = "cc-cedict"))]
decompress_data!(WORDS_DATA, &[], "dict.words");

pub fn load_dictionary() -> LinderaResult<Dictionary> {
    Ok(Dictionary {
        dict: prefix_dict(),
        cost_matrix: connection(),
        char_definitions: char_def()?,
        unknown_dictionary: unknown_dict()?,
    })
}

pub fn char_def() -> LinderaResult<CharacterDefinition> {
    let char_def_data = &CHAR_DEFINITION_DATA;
    CharacterDefinition::load(char_def_data)
}

pub fn connection() -> ConnectionCostMatrix {
    let connection_data = &CONNECTION_DATA;
    #[cfg(feature = "compress")]
    {
        ConnectionCostMatrix::load(connection_data)
    }
    #[cfg(not(feature = "compress"))]
    {
        ConnectionCostMatrix::load_static(connection_data)
    }
}

pub fn prefix_dict() -> PrefixDictionary {
    let cc_cedict_data = &CC_CEDICT_DATA;
    let cc_cedict_vals = &CC_CEDICT_VALS;
    let words_idx_data = &WORDS_IDX_DATA;
    let words_data = &WORDS_DATA;
    PrefixDictionary::load(cc_cedict_data, cc_cedict_vals, words_idx_data, words_data)
}

pub fn unknown_dict() -> LinderaResult<UnknownDictionary> {
    let unknown_data = &UNKNOWN_DATA;
    UnknownDictionary::load(unknown_data)
}
