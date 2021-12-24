use std::env;

use lindera_compress::decompress;
use lindera_core::character_definition::CharacterDefinitions;
use lindera_core::connection::ConnectionCostMatrix;
use lindera_core::prefix_dict::PrefixDict;
use lindera_core::unknown_dictionary::UnknownDictionary;
use lindera_core::LinderaResult;
use once_cell::sync::Lazy;

const CHAR_DEFINITION_DATA: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
    let compressed_data = bincode::deserialize_from(
        &include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/char_def.bin"))[..],
    )
    .expect("invalid file format char_def.bin");
    decompress(compressed_data).expect("invalid file format char_def.bin")
});

const CONNECTION_DATA: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
    let compressed_data = bincode::deserialize_from(
        &include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/matrix.mtx"))[..],
    )
    .expect("invalid file format matrix.mtx");
    decompress(compressed_data).expect("invalid file format matrix.mtx")
});

const IPADIC_DATA: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
    let compressed_data = bincode::deserialize_from(
        &include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/dict.da"))[..],
    )
    .expect("invalid file format dict.da");
    decompress(compressed_data).expect("invalid file format dict.da")
});

const IPADIC_VALS: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
    let compressed_data = bincode::deserialize_from(
        &include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/dict.vals"))[..],
    )
    .expect("invalid file format dict.vals");
    decompress(compressed_data).expect("invalid file format dict.vals")
});

const UNKNOWN_DATA: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
    let compressed_data = bincode::deserialize_from(
        &include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/unk.bin"))[..],
    )
    .expect("invalid file format unk.bin");
    decompress(compressed_data).expect("invalid file format unk.bin")
});

const WORDS_IDX_DATA: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
    let compressed_data = bincode::deserialize_from(
        &include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/dict.wordsidx"))[..],
    )
    .expect("invalid file format unk.bin");
    decompress(compressed_data).expect("invalid file format unk.bin")
});

const WORDS_DATA: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
    let compressed_data = bincode::deserialize_from(
        &include_bytes!(concat!(env!("OUT_DIR"), "/lindera-ipadic/dict.words"))[..],
    )
    .expect("invalid file format unk.bin");
    decompress(compressed_data).expect("invalid file format unk.bin")
});

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
