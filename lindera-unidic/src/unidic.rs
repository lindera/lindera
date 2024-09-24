use std::borrow::Cow;
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
    let unidic_data = &UNIDIC_DATA;
    let unidic_vals = &UNIDIC_VALS;
    PrefixDictionary::from_static_slice(unidic_data, unidic_vals)
}

pub fn unknown_dict() -> LinderaResult<UnknownDictionary> {
    let unknown_data = &UNKNOWN_DATA;
    UnknownDictionary::load(unknown_data)
}

pub fn words_idx_data() -> Cow<'static, [u8]> {
    let words_idx_data = &WORDS_IDX_DATA;
    #[cfg(feature = "compress")]
    {
        Cow::Owned(words_idx_data.to_vec())
    }
    #[cfg(not(feature = "compress"))]
    {
        Cow::Borrowed(words_idx_data)
    }
}

pub fn words_data() -> Cow<'static, [u8]> {
    let words_data = &WORDS_DATA;
    #[cfg(feature = "compress")]
    {
        Cow::Owned(words_data.to_vec())
    }
    #[cfg(not(feature = "compress"))]
    {
        Cow::Borrowed(words_data)
    }
}
