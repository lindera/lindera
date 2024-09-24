#[cfg(feature = "ipadic-neologd")]
use std::env;

use lindera_core::dictionary::character_definition::CharacterDefinition;
use lindera_core::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_core::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_core::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_core::dictionary::Dictionary;
use lindera_core::LinderaResult;

#[cfg(feature = "compress")]
use lindera_core::decompress::decompress;

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

#[cfg(feature = "ipadic-neologd")]
decompress_data!(
    CHAR_DEFINITION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic-neologd/char_def.bin"
    )),
    "char_def.bin"
);
#[cfg(not(feature = "ipadic-neologd"))]
decompress_data!(CHAR_DEFINITION_DATA, &[], "char_def.bin");

#[cfg(feature = "ipadic-neologd")]
decompress_data!(
    CONNECTION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic-neologd/matrix.mtx"
    )),
    "matrix.mtx"
);
#[cfg(not(feature = "ipadic-neologd"))]
decompress_data!(CONNECTION_DATA, &[], "matrix.mtx");

#[cfg(feature = "ipadic-neologd")]
decompress_data!(
    IPADIC_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic-neologd/dict.da"
    )),
    "dict.da"
);
#[cfg(not(feature = "ipadic-neologd"))]
decompress_data!(IPADIC_DATA, &[], "dict.da");

#[cfg(feature = "ipadic-neologd")]
decompress_data!(
    IPADIC_VALS,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic-neologd/dict.vals"
    )),
    "dict.vals"
);
#[cfg(not(feature = "ipadic-neologd"))]
decompress_data!(IPADIC_VALS, &[], "dict.vals");

#[cfg(feature = "ipadic-neologd")]
decompress_data!(
    UNKNOWN_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic-neologd/unk.bin"
    )),
    "unk.bin"
);
#[cfg(not(feature = "ipadic-neologd"))]
decompress_data!(UNKNOWN_DATA, &[], "unk.bin");

#[cfg(feature = "ipadic-neologd")]
decompress_data!(
    WORDS_IDX_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic-neologd/dict.wordsidx"
    )),
    "dict.wordsidx"
);
#[cfg(not(feature = "ipadic-neologd"))]
decompress_data!(WORDS_IDX_DATA, &[], "dict.wordsidx");

#[cfg(feature = "ipadic-neologd")]
decompress_data!(
    WORDS_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic-neologd/dict.words"
    )),
    "dict.words"
);
#[cfg(not(feature = "ipadic-neologd"))]
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
    let ipadic_data = &IPADIC_DATA;
    let ipadic_vals = &IPADIC_VALS;
    let words_idx_data = &WORDS_IDX_DATA;
    let words_data = &WORDS_DATA;
    PrefixDictionary::from_static_slice(ipadic_data, ipadic_vals, words_idx_data, words_data)
}

pub fn unknown_dict() -> LinderaResult<UnknownDictionary> {
    let unknown_data = &UNKNOWN_DATA;
    UnknownDictionary::load(unknown_data)
}
