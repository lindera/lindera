#[cfg(feature = "unidic")]
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

#[cfg(feature = "unidic")]
decompress_data!(
    CHAR_DEFINITION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-unidic/char_def.bin"
    )),
    "char_def.bin"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(CHAR_DEFINITION_DATA, &[], "char_def.bin");

#[cfg(feature = "unidic")]
decompress_data!(
    CONNECTION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-unidic/matrix.mtx"
    )),
    "matrix.mtx"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(CONNECTION_DATA, &[], "matrix.mtx");

#[cfg(feature = "unidic")]
decompress_data!(
    UNIDIC_DATA,
    include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-unidic/dict.da")),
    "dict.da"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(UNIDIC_DATA, &[], "dict.da");

#[cfg(feature = "unidic")]
decompress_data!(
    UNIDIC_VALS,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-unidic/dict.vals"
    )),
    "dict.vals"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(UNIDIC_VALS, &[], "dict.vals");

#[cfg(feature = "unidic")]
decompress_data!(
    UNKNOWN_DATA,
    include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-unidic/unk.bin")),
    "unk.bin"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(UNKNOWN_DATA, &[], "unk.bin");

#[cfg(feature = "unidic")]
decompress_data!(
    WORDS_IDX_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-unidic/dict.wordsidx"
    )),
    "dict.wordsidx"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(WORDS_IDX_DATA, &[], "dict.wordsidx");

#[cfg(feature = "unidic")]
decompress_data!(
    WORDS_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-unidic/dict.words"
    )),
    "dict.words"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(WORDS_DATA, &[], "dict.words");

pub fn load() -> LinderaResult<Dictionary> {
    Ok(Dictionary {
        prefix_dictionary: load_prefix_dictionary(),
        connection_cost_matrix: load_connection_cost_matrix(),
        character_definition: load_character_definition()?,
        unknown_dictionary: load_unknown_dictionary()?,
    })
}

fn load_character_definition() -> LinderaResult<CharacterDefinition> {
    let char_def_data = &CHAR_DEFINITION_DATA;
    CharacterDefinition::load(char_def_data)
}

fn load_connection_cost_matrix() -> ConnectionCostMatrix {
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

fn load_prefix_dictionary() -> PrefixDictionary {
    let unidic_data = &UNIDIC_DATA;
    let unidic_vals = &UNIDIC_VALS;
    let words_idx_data = &WORDS_IDX_DATA;
    let words_data = &WORDS_DATA;
    PrefixDictionary::load(unidic_data, unidic_vals, words_idx_data, words_data)
}

fn load_unknown_dictionary() -> LinderaResult<UnknownDictionary> {
    let unknown_data = &UNKNOWN_DATA;
    UnknownDictionary::load(unknown_data)
}
