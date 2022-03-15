use std::fs;
use std::path::PathBuf;

use lindera_core::character_definition::CharacterDefinitions;
use lindera_core::connection::ConnectionCostMatrix;
use lindera_core::dictionary::Dictionary;
use lindera_core::error::LinderaErrorKind;
use lindera_core::prefix_dict::PrefixDict;
use lindera_core::unknown_dictionary::UnknownDictionary;
use lindera_core::LinderaResult;

fn read_file(path: PathBuf) -> LinderaResult<Vec<u8>> {
    fs::read(path).map_err(|e| LinderaErrorKind::Io.with_error(e))
}

pub fn load_dictionary(path: PathBuf) -> LinderaResult<Dictionary> {
    Ok(Dictionary {
        dict: prefix_dict(path.clone())?,
        cost_matrix: connection(path.clone())?,
        char_definitions: char_def(path.clone())?,
        unknown_dictionary: unknown_dict(path.clone())?,
        words_idx_data: words_idx_data(path.clone())?,
        words_data: words_data(path)?,
    })
}

pub fn char_def(dir: PathBuf) -> LinderaResult<CharacterDefinitions> {
    let path = dir.join("char_def.bin");
    let data = read_file(path)?;

    CharacterDefinitions::load(data.as_slice())
}

pub fn connection(dir: PathBuf) -> LinderaResult<ConnectionCostMatrix> {
    let path = dir.join("matrix.mtx");
    let data = read_file(path)?;

    Ok(ConnectionCostMatrix::load(data.as_slice()))
}

pub fn prefix_dict(dir: PathBuf) -> LinderaResult<PrefixDict> {
    let unidic_data_path = dir.join("dict.da");
    let unidic_data = read_file(unidic_data_path)?;

    let unidic_vals_path = dir.join("dict.vals");
    let unidic_vals = read_file(unidic_vals_path)?;

    Ok(PrefixDict::from_static_slice(
        unidic_data.as_slice(),
        unidic_vals.as_slice(),
    ))
}

pub fn unknown_dict(dir: PathBuf) -> LinderaResult<UnknownDictionary> {
    let path = dir.join("unk.bin");
    let data = read_file(path)?;

    UnknownDictionary::load(data.as_slice())
}

pub fn words_idx_data(dir: PathBuf) -> LinderaResult<Vec<u8>> {
    let path = dir.join("dict.wordsidx");
    read_file(path)
}

pub fn words_data(dir: PathBuf) -> LinderaResult<Vec<u8>> {
    let path = dir.join("dict.words");
    read_file(path)
}
