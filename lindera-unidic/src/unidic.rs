#[cfg(feature = "unidic")]
use std::env;

#[cfg(feature = "compress")]
use lindera_dictionary::decompress::decompress;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::LinderaResult;

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
    DA_DATA,
    include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-unidic/dict.da")),
    "dict.da"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(DA_DATA, &[], "dict.da");

#[cfg(feature = "unidic")]
decompress_data!(
    VALS_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-unidic/dict.vals"
    )),
    "dict.vals"
);
#[cfg(not(feature = "unidic"))]
decompress_data!(VALS_DATA, &[], "dict.vals");

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
    let da_data = &DA_DATA;
    let vals_data = &VALS_DATA;
    let words_idx_data = &WORDS_IDX_DATA;
    let words_data = &WORDS_DATA;
    let connection_data = &CONNECTION_DATA;
    let char_definition = &CHAR_DEFINITION_DATA;
    let unknown_data = &UNKNOWN_DATA;

    Ok(Dictionary {
        prefix_dictionary: PrefixDictionary::load(da_data, vals_data, words_idx_data, words_data),
        connection_cost_matrix: ConnectionCostMatrix::load_static(connection_data),
        character_definition: CharacterDefinition::load(char_definition)?,
        unknown_dictionary: UnknownDictionary::load(unknown_data)?,
    })
}
