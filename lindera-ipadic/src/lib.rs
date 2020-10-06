use lindera_core::core::character_definition::CharacterDefinitions;
use lindera_core::core::connection::ConnectionCostMatrix;
use lindera_core::core::prefix_dict::PrefixDict;
use lindera_core::core::unknown_dictionary::UnknownDictionary;

const CHAR_DEFINITION_DATA: &'static [u8] = include_bytes!("../lindera-ipadic/char_def.bin");
const CONNECTION_DATA: &'static [u8] = include_bytes!("../lindera-ipadic/matrix.mtx");
const IPADIC_DATA: &'static [u8] = include_bytes!("../lindera-ipadic/dict.da");
const IPADIC_VALS: &'static [u8] = include_bytes!("../lindera-ipadic/dict.vals");
const UNKNOWN_DATA: &'static [u8] = include_bytes!("../lindera-ipadic/unk.bin");
const WORDS_IDX_DATA: &'static [u8] = include_bytes!("../lindera-ipadic/dict.wordsidx");
const WORDS_DATA: &'static [u8] = include_bytes!("../lindera-ipadic/dict.words");

pub fn char_def() -> CharacterDefinitions {
    CharacterDefinitions::load(CHAR_DEFINITION_DATA)
}

pub fn connection() -> ConnectionCostMatrix {
    ConnectionCostMatrix::load(CONNECTION_DATA)
}

pub fn prefix_dict() -> PrefixDict {
    PrefixDict::from_static_slice(IPADIC_DATA, IPADIC_VALS).unwrap()
}

pub fn unknown_dict() -> UnknownDictionary {
    UnknownDictionary::load(UNKNOWN_DATA)
}

pub fn words_idx_data() -> Vec<u8> {
    WORDS_IDX_DATA.to_vec()
}

pub fn words_data() -> Vec<u8> {
    WORDS_DATA.to_vec()
}
