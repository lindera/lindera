use std::borrow::Cow;
#[cfg(feature = "ko-dic")]
use std::env;

#[cfg(feature = "compress")]
use lindera_core::decompress::decompress;
use lindera_core::dictionary::character_definition::CharacterDefinitions;
use lindera_core::dictionary::connection::ConnectionCostMatrix;
use lindera_core::dictionary::prefix_dict::PrefixDict;
use lindera_core::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_core::dictionary::Dictionary;
use lindera_core::LinderaResult;

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

#[cfg(feature = "ko-dic")]
decompress_data!(
    CHAR_DEFINITION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ko-dic/char_def.bin"
    )),
    "char_def.bin"
);
#[cfg(not(feature = "ko-dic"))]
decompress_data!(CHAR_DEFINITION_DATA, &[], "char_def.bin");

#[cfg(feature = "ko-dic")]
decompress_data!(
    CONNECTION_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ko-dic/matrix.mtx"
    )),
    "matrix.mtx"
);
#[cfg(not(feature = "ko-dic"))]
decompress_data!(CONNECTION_DATA, &[], "matrix.mtx");

#[cfg(feature = "ko-dic")]
decompress_data!(
    KO_DIC_DATA,
    include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-ko-dic/dict.da")),
    "dict.da"
);
#[cfg(not(feature = "ko-dic"))]
decompress_data!(KO_DIC_DATA, &[], "dict.da");

#[cfg(feature = "ko-dic")]
decompress_data!(
    KO_DIC_VALS,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ko-dic/dict.vals"
    )),
    "dict.vals"
);
#[cfg(not(feature = "ko-dic"))]
decompress_data!(KO_DIC_VALS, &[], "dict.vals");

#[cfg(feature = "ko-dic")]
decompress_data!(
    UNKNOWN_DATA,
    include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-ko-dic/unk.bin")),
    "unk.bin"
);
#[cfg(not(feature = "ko-dic"))]
decompress_data!(UNKNOWN_DATA, &[], "unk.bin");

#[cfg(feature = "ko-dic")]
decompress_data!(
    WORDS_IDX_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ko-dic/dict.wordsidx"
    )),
    "dict.wordsidx"
);
#[cfg(not(feature = "ko-dic"))]
decompress_data!(WORDS_IDX_DATA, &[], "dict.wordsidx");

#[cfg(feature = "ko-dic")]
decompress_data!(
    WORDS_DATA,
    include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ko-dic/dict.words"
    )),
    "dict.words"
);
#[cfg(not(feature = "ko-dic"))]
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
    PrefixDict::from_static_slice(&KO_DIC_DATA, &KO_DIC_VALS)
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
