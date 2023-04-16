use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use lindera_cc_cedict_builder::cc_cedict_builder::CcCedictBuilder;
use lindera_core::{
    character_definition::CharacterDefinitions,
    connection::ConnectionCostMatrix,
    dictionary::Dictionary,
    dictionary_builder::DictionaryBuilder,
    error::{LinderaError, LinderaErrorKind},
    prefix_dict::PrefixDict,
    unknown_dictionary::UnknownDictionary,
    user_dictionary::UserDictionary,
    LinderaResult,
};
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;
use lindera_ko_dic_builder::ko_dic_builder::KoDicBuilder;
use lindera_unidic_builder::unidic_builder::UnidicBuilder;

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

/// Dictionary config
///
/// Use this if you want to use a dictionary when tokenizing.
///
/// Either `kind` or `path` must be specified.
///
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DictionaryConfig {
    /// Specify the kind of dictionary (IPADIC, UniDic, ko-dic, CC-CEDICT) if a self-contained dictionary is used for tokenization.
    pub kind: Option<DictionaryKind>,
    /// Specifies the path to a pre-built external dictionary if one is used.
    pub path: Option<PathBuf>,
}

/// User dictionary config
///
/// Use this if you want to use a user dictionary when tokenizing.
///
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserDictionaryConfig {
    /// Path to the user dictionary file.
    pub path: PathBuf,
    /// If the user dictionary was in CSV format, specify the dictionary type (IPADIC, UniDic, ko-dic or CC-CEDICT).
    pub kind: Option<DictionaryKind>,
}

pub fn resolve_builder(
    dictionary_type: DictionaryKind,
) -> LinderaResult<Box<dyn DictionaryBuilder>> {
    match dictionary_type {
        DictionaryKind::IPADIC => Ok(Box::new(IpadicBuilder::new())),
        DictionaryKind::UniDic => Ok(Box::new(UnidicBuilder::new())),
        DictionaryKind::KoDic => Ok(Box::new(KoDicBuilder::new())),
        DictionaryKind::CcCedict => Ok(Box::new(CcCedictBuilder::new())),
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

pub fn load_dictionary_from_config(
    dictionary_config: DictionaryConfig,
) -> LinderaResult<Dictionary> {
    match dictionary_config.kind {
        Some(kind) => {
            // The dictionary specified by the feature flag will be loaded.
            load_dictionary_from_kind(kind)
        }
        None => {
            match dictionary_config.path {
                Some(path) => {
                    // load external dictionary from path
                    load_dictionary(path)
                }
                None => Err(LinderaErrorKind::Args
                    .with_error(anyhow::anyhow!("Dictionary must be specified"))),
            }
        }
    }
}

pub fn build_dictionary(
    dictionary_type: DictionaryKind,
    input_dir: &Path,
    output_dir: &Path,
) -> LinderaResult<()> {
    resolve_builder(dictionary_type)?.build_dictionary(input_dir, output_dir)
}

pub fn build_user_dictionary(
    dictionary_type: DictionaryKind,
    input_file: &Path,
    output_dir: &Path,
) -> LinderaResult<()> {
    let output_file = if let Some(filename) = input_file.file_name() {
        let mut output_file = Path::new(output_dir).join(filename);
        output_file.set_extension("bin");
        output_file
    } else {
        return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!("failed to get filename")));
    };

    resolve_builder(dictionary_type)?.build_user_dictionary(input_file, &output_file)
}

pub fn build_user_dictionary_from_csv(
    kind: DictionaryKind,
    path: PathBuf,
) -> LinderaResult<UserDictionary> {
    resolve_builder(kind)?
        .build_user_dict(&path)
        .map_err(|err| LinderaErrorKind::DictionaryBuildError.with_error(err))
}

pub fn load_user_dictionary_from_bin(data: &[u8]) -> LinderaResult<UserDictionary> {
    UserDictionary::load(data)
}

pub fn load_user_dictionary(
    dictionary_config: UserDictionaryConfig,
) -> LinderaResult<UserDictionary> {
    match dictionary_config.path.extension() {
        Some(ext) => match ext.to_str() {
            Some("csv") => match dictionary_config.kind {
                Some(kind) => build_user_dictionary_from_csv(kind, dictionary_config.path),
                None => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                    "Dictionary type must be specified if CSV file specified"
                ))),
            },
            Some("bin") => load_user_dictionary_from_bin(&read_file(dictionary_config.path)?),
            _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                "Invalid user dictionary source file extension"
            ))),
        },
        None => Err(LinderaErrorKind::Args
            .with_error(anyhow::anyhow!("Invalid user dictionary source file"))),
    }
}
