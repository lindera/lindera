use std::borrow::Cow;
#[cfg(feature = "ipadic")]
use std::env;

use lindera_core::dictionary::character_definition::CharacterDefinitions;
use lindera_core::dictionary::connection::ConnectionCostMatrix;
use lindera_core::dictionary::prefix_dict::PrefixDict;
use lindera_core::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_core::dictionary::Dictionary;
use lindera_core::LinderaResult;

#[cfg(feature = "compress")]
use lindera_core::decompress::decompress;

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

#[cfg(feature = "ipadic")]
decompress_data!(
    CHAR_DEFINITION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/char_def.bin"
    )),
    "char_def.bin"
);
#[cfg(not(feature = "ipadic"))]
decompress_data!(CHAR_DEFINITION_DATA, &[], "char_def.bin");

#[cfg(feature = "ipadic")]
decompress_data!(
    CONNECTION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/matrix.mtx"
    )),
    "matrix.mtx"
);
#[cfg(not(feature = "ipadic"))]
decompress_data!(CONNECTION_DATA, &[], "matrix.mtx");

#[cfg(feature = "ipadic")]
decompress_data!(
    IPADIC_DATA,
    include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-ipadic/dict.da")),
    "dict.da"
);
#[cfg(not(feature = "ipadic"))]
decompress_data!(IPADIC_DATA, &[], "dict.da");

#[cfg(feature = "ipadic")]
decompress_data!(
    IPADIC_VALS,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/dict.vals"
    )),
    "dict.vals"
);
#[cfg(not(feature = "ipadic"))]
decompress_data!(IPADIC_VALS, &[], "dict.vals");

#[cfg(feature = "ipadic")]
decompress_data!(
    UNKNOWN_DATA,
    include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-ipadic/unk.bin")),
    "unk.bin"
);
#[cfg(not(feature = "ipadic"))]
decompress_data!(UNKNOWN_DATA, &[], "unk.bin");

#[cfg(feature = "ipadic")]
decompress_data!(
    WORDS_IDX_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/dict.wordsidx"
    )),
    "dict.wordsidx"
);
#[cfg(not(feature = "ipadic"))]
decompress_data!(WORDS_IDX_DATA, &[], "dict.wordsidx");

#[cfg(feature = "ipadic")]
decompress_data!(
    WORDS_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/dict.words"
    )),
    "dict.words"
);
#[cfg(not(feature = "ipadic"))]
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
    #[allow(clippy::needless_borrow)]
    CharacterDefinitions::load(&CHAR_DEFINITION_DATA)
}

pub fn connection() -> ConnectionCostMatrix {
    #[cfg(feature = "compress")]
    {
        ConnectionCostMatrix::load(&CONNECTION_DATA)
    }
    #[cfg(not(feature = "compress"))]
    {
        ConnectionCostMatrix::load_static(CONNECTION_DATA)
    }
}

pub fn prefix_dict() -> PrefixDict {
    #[allow(clippy::needless_borrow)]
    PrefixDict::from_static_slice(&IPADIC_DATA, &IPADIC_VALS)
}

pub fn unknown_dict() -> LinderaResult<UnknownDictionary> {
    #[allow(clippy::needless_borrow)]
    UnknownDictionary::load(&UNKNOWN_DATA)
}

pub fn words_idx_data() -> Cow<'static, [u8]> {
    #[cfg(feature = "compress")]
    {
        Cow::Owned(WORDS_IDX_DATA.to_vec())
    }
    #[cfg(not(feature = "compress"))]
    {
        Cow::Borrowed(WORDS_IDX_DATA)
    }
}

pub fn words_data() -> Cow<'static, [u8]> {
    #[cfg(feature = "compress")]
    {
        Cow::Owned(WORDS_DATA.to_vec())
    }
    #[cfg(not(feature = "compress"))]
    {
        Cow::Borrowed(WORDS_DATA)
    }
}
