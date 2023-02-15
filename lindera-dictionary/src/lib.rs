use std::{fs, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use lindera_core::{
    character_definition::CharacterDefinitions,
    connection::ConnectionCostMatrix,
    dictionary::Dictionary,
    error::{LinderaError, LinderaErrorKind},
    prefix_dict::PrefixDict,
    unknown_dictionary::UnknownDictionary,
    LinderaResult,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryKind {
    #[serde(rename = "ipadic")]
    IPADIC,
    #[serde(rename = "unidic")]
    UniDic,
    #[serde(rename = "ko-dic")]
    KoDic,
    #[serde(rename = "cc-cedict")]
    CcCedict,
}

impl FromStr for DictionaryKind {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryKind, Self::Err> {
        match input {
            "ipadic" => Ok(DictionaryKind::IPADIC),
            "unidic" => Ok(DictionaryKind::UniDic),
            "ko-dic" => Ok(DictionaryKind::KoDic),
            "cc-cedict" => Ok(DictionaryKind::CcCedict),
            _ => Err(LinderaErrorKind::DictionaryKindError
                .with_error(anyhow::anyhow!("Invalid dictionary kind: {}", input))),
        }
    }
}

fn read_file(path: PathBuf) -> LinderaResult<Vec<u8>> {
    fs::read(path).map_err(|e| LinderaErrorKind::Io.with_error(e))
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

pub fn connection(dir: PathBuf) -> LinderaResult<ConnectionCostMatrix> {
    let path = dir.join("matrix.mtx");
    let data = read_file(path)?;

    Ok(ConnectionCostMatrix::load(data.as_slice()))
}

pub fn char_def(dir: PathBuf) -> LinderaResult<CharacterDefinitions> {
    let path = dir.join("char_def.bin");
    let data = read_file(path)?;

    CharacterDefinitions::load(data.as_slice())
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

pub fn load_dictionary_from_kind(kind: DictionaryKind) -> LinderaResult<Dictionary> {
    // The dictionary specified by the feature flag will be loaded.
    match kind {
        #[cfg(feature = "ipadic")]
        DictionaryKind::IPADIC => lindera_ipadic::load_dictionary()
            .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
        #[cfg(feature = "unidic")]
        DictionaryKind::UniDic => lindera_unidic::load_dictionary()
            .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
        #[cfg(feature = "ko-dic")]
        DictionaryKind::KoDic => lindera_ko_dic::load_dictionary()
            .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
        #[cfg(feature = "cc-cedict")]
        DictionaryKind::CcCedict => lindera_cc_cedict::load_dictionary()
            .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
        #[allow(unreachable_patterns)]
        _ => Err(LinderaErrorKind::Args
            .with_error(anyhow::anyhow!("Invalid dictionary type: {:?}", kind))),
    }
}
