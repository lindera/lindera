use std::env;

#[cfg(feature = "smallbinary")]
use lindera_compress::decompress;
use lindera_core::character_definition::CharacterDefinitions;
use lindera_core::connection::ConnectionCostMatrix;
use lindera_core::prefix_dict::PrefixDict;
use lindera_core::unknown_dictionary::UnknownDictionary;
use lindera_core::LinderaResult;

macro_rules! decompress_or_raw {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "smallbinary")]
        const $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            let compressed_data = bincode::deserialize_from(&$bytes[..])
                .expect(concat!("invalid file format ", $filename));
            decompress(compressed_data).expect("invalid file format $filename")
        });
        #[cfg(not(feature = "smallbinary"))]
        const $name: &'static [u8] = $bytes;
    };
}

decompress_or_raw!(
    CHAR_DEFINITION_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/char_def.bin")),
    "char_def.bin"
);
decompress_or_raw!(
    CONNECTION_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/matrix.mtx")),
    "matrix.mtx"
);
decompress_or_raw!(
    IPADIC_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/dict.da")),
    "dict.da"
);
decompress_or_raw!(
    IPADIC_VALS,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/dict.vals")),
    "dict.vals"
);
decompress_or_raw!(
    UNKNOWN_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/unk.bin")),
    "unk.bin"
);
decompress_or_raw!(
    WORDS_IDX_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/dict.wordsidx")),
    "dict.wordsidx"
);
decompress_or_raw!(
    WORDS_DATA,
    include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/dict.words")),
    "dict.words"
);

pub fn char_def() -> LinderaResult<CharacterDefinitions> {
    CharacterDefinitions::load(&CHAR_DEFINITION_DATA)
}

pub fn connection() -> ConnectionCostMatrix {
    ConnectionCostMatrix::load(&CONNECTION_DATA)
}

pub fn prefix_dict() -> PrefixDict {
    PrefixDict::from_static_slice(&IPADIC_DATA, &IPADIC_VALS)
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
