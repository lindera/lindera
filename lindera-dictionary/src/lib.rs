use lindera_core::core::character_definition::CharacterDefinitions;
use lindera_core::core::connection::ConnectionCostMatrix;
use lindera_core::core::prefix_dict::PrefixDict;
use lindera_core::core::unknown_dictionary::UnknownDictionary;
use std::fs;
use std::path::Path;

fn read_file(file: &str) -> Vec<u8> {
    let mut data = Vec::new();
    match fs::read(file) {
        Ok(_data) => data = _data,
        Err(e) => println!("{}", e.to_string()),
    }
    data
}

pub fn char_def(dir: &str) -> CharacterDefinitions {
    let path = Path::new(dir).join("char_def.bin");
    let data = read_file(path.to_str().unwrap());

    CharacterDefinitions::load(data.as_slice())
}

pub fn connection(dir: &str) -> ConnectionCostMatrix {
    let path = Path::new(dir).join("matrix.mtx");
    let data = read_file(path.to_str().unwrap());

    ConnectionCostMatrix::load(data.as_slice())
}

pub fn prefix_dict(dir: &str) -> PrefixDict {
    let unidic_data_path = Path::new(dir).join("dict.da");
    let unidic_data = read_file(unidic_data_path.to_str().unwrap());

    let unidic_vals_path = Path::new(dir).join("dict.vals");
    let unidic_vals = read_file(unidic_vals_path.to_str().unwrap());

    PrefixDict::from_static_slice(unidic_data.as_slice(), unidic_vals.as_slice()).unwrap()
}

pub fn unknown_dict(dir: &str) -> UnknownDictionary {
    let path = Path::new(dir).join("unk.bin");
    let data = read_file(path.to_str().unwrap());

    UnknownDictionary::load(data.as_slice())
}

pub fn words_idx_data(dir: &str) -> Vec<u8> {
    let path = Path::new(dir).join("dict.wordsidx");
    let data = read_file(path.to_str().unwrap());

    data
}

pub fn words_data(dir: &str) -> Vec<u8> {
    let path = Path::new(dir).join("dict.words");
    let data = read_file(path.to_str().unwrap());

    data
}
