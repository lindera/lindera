use std::path::PathBuf;
use std::str::FromStr;

use lindera_core::util::read_file;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use lindera_core::dictionary::character_definition::CharacterDefinition;
use lindera_core::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_core::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_core::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_core::dictionary::{Dictionary, UserDictionary};
use lindera_core::dictionary_builder::cc_cedict::CcCedictBuilder;
use lindera_core::dictionary_builder::ipadic::IpadicBuilder;
use lindera_core::dictionary_builder::ipadic_neologd::IpadicNeologdBuilder;
use lindera_core::dictionary_builder::ko_dic::KoDicBuilder;
use lindera_core::dictionary_builder::unidic::UnidicBuilder;
use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::error::{LinderaError, LinderaErrorKind};
use lindera_core::LinderaResult;

#[derive(Debug, Clone, EnumIter, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryKind {
    #[serde(rename = "ipadic")]
    IPADIC,
    #[serde(rename = "ipadic-neologd")]
    IPADICNEologd,
    #[serde(rename = "unidic")]
    UniDic,
    #[serde(rename = "ko-dic")]
    KoDic,
    #[serde(rename = "cc-cedict")]
    CcCedict,
}

impl DictionaryKind {
    pub fn variants() -> Vec<DictionaryKind> {
        DictionaryKind::iter().collect::<Vec<_>>()
    }

    pub fn contained_variants() -> Vec<DictionaryKind> {
        DictionaryKind::variants()
            .into_iter()
            .filter(|kind| match kind {
                DictionaryKind::IPADIC => cfg!(feature = "ipadic"),
                DictionaryKind::IPADICNEologd => cfg!(feature = "ipadic-neologd"),
                DictionaryKind::UniDic => cfg!(feature = "unidic"),
                DictionaryKind::KoDic => cfg!(feature = "ko-dic"),
                DictionaryKind::CcCedict => cfg!(feature = "cc-cedict"),
            })
            .collect::<Vec<_>>()
    }

    pub fn as_str(&self) -> &str {
        match self {
            DictionaryKind::IPADIC => "ipadic",
            DictionaryKind::IPADICNEologd => "ipadic-neologd",
            DictionaryKind::UniDic => "unidic",
            DictionaryKind::KoDic => "ko-dic",
            DictionaryKind::CcCedict => "cc-cedict",
        }
    }
}

impl FromStr for DictionaryKind {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryKind, Self::Err> {
        match input {
            "ipadic" => Ok(DictionaryKind::IPADIC),
            "ipadic-neologd" => Ok(DictionaryKind::IPADICNEologd),
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
        DictionaryKind::IPADICNEologd => Ok(Box::new(IpadicNeologdBuilder::new())),
        DictionaryKind::UniDic => Ok(Box::new(UnidicBuilder::new())),
        DictionaryKind::KoDic => Ok(Box::new(KoDicBuilder::new())),
        DictionaryKind::CcCedict => Ok(Box::new(CcCedictBuilder::new())),
    }
}

fn load_prefix_dictionary(dir: PathBuf) -> LinderaResult<PrefixDictionary> {
    let dict_da_path = dir.join("dict.da");
    let dict_da = read_file(dict_da_path.as_path())?;

    let dict_vals_path = dir.join("dict.vals");
    let dict_vals = read_file(dict_vals_path.as_path())?;

    let dict_wordsidx_path = dir.join("dict.wordsidx");
    let dict_wordsidx = read_file(dict_wordsidx_path.as_path())?;

    let dict_words_path = dir.join("dict.words");
    let dict_words = read_file(dict_words_path.as_path())?;

    Ok(PrefixDictionary::load(
        dict_da.as_slice(),
        dict_vals.as_slice(),
        dict_wordsidx.as_slice(),
        dict_words.as_slice(),
    ))
}

fn load_connection_cost_matrix(dir: PathBuf) -> LinderaResult<ConnectionCostMatrix> {
    let path = dir.join("matrix.mtx");
    let data = read_file(path.as_path())?;

    Ok(ConnectionCostMatrix::load(data.as_slice()))
}

fn load_character_definition(dir: PathBuf) -> LinderaResult<CharacterDefinition> {
    let path = dir.join("char_def.bin");
    let data = read_file(path.as_path())?;

    CharacterDefinition::load(data.as_slice())
}

fn load_unknown_dictionary(dir: PathBuf) -> LinderaResult<UnknownDictionary> {
    let path = dir.join("unk.bin");
    let data = read_file(path.as_path())?;

    UnknownDictionary::load(data.as_slice())
}

pub fn load_dictionary_from_path(path: PathBuf) -> LinderaResult<Dictionary> {
    Ok(Dictionary {
        prefix_dictionary: load_prefix_dictionary(path.clone())?,
        connection_cost_matrix: load_connection_cost_matrix(path.clone())?,
        character_definition: load_character_definition(path.clone())?,
        unknown_dictionary: load_unknown_dictionary(path.clone())?,
    })
}

pub fn load_dictionary_from_kind(kind: DictionaryKind) -> LinderaResult<Dictionary> {
    // The dictionary specified by the feature flag will be loaded.
    match kind {
        #[cfg(feature = "ipadic")]
        DictionaryKind::IPADIC => lindera_ipadic::ipadic::load()
            .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
        #[cfg(feature = "ipadic-neologd")]
        DictionaryKind::IPADICNEologd => lindera_ipadic_neologd::ipadic_neologd::load()
            .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
        #[cfg(feature = "unidic")]
        DictionaryKind::UniDic => lindera_unidic::unidic::load()
            .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
        #[cfg(feature = "ko-dic")]
        DictionaryKind::KoDic => lindera_ko_dic::ko_dic::load()
            .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
        #[cfg(feature = "cc-cedict")]
        DictionaryKind::CcCedict => lindera_cc_cedict::cc_cedict::load()
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
                    load_dictionary_from_path(path)
                }
                None => Err(LinderaErrorKind::Args
                    .with_error(anyhow::anyhow!("Dictionary must be specified"))),
            }
        }
    }
}

pub fn load_user_dictionary_from_csv(
    kind: DictionaryKind,
    path: PathBuf,
) -> LinderaResult<UserDictionary> {
    let builder = resolve_builder(kind)?;
    builder
        .build_user_dict(path.as_path())
        .map_err(|err| LinderaErrorKind::DictionaryBuildError.with_error(err))
}

pub fn load_user_dictionary_from_bin(path: PathBuf) -> LinderaResult<UserDictionary> {
    UserDictionary::load(&read_file(path.as_path())?)
}

pub fn load_user_dictionary_from_config(
    dictionary_config: UserDictionaryConfig,
) -> LinderaResult<UserDictionary> {
    match dictionary_config.path.extension() {
        Some(ext) => match ext.to_str() {
            Some("csv") => match dictionary_config.kind {
                Some(kind) => load_user_dictionary_from_csv(kind, dictionary_config.path),
                None => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                    "Dictionary type must be specified if CSV file specified"
                ))),
            },
            Some("bin") => load_user_dictionary_from_bin(dictionary_config.path),
            _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                "Invalid user dictionary source file extension"
            ))),
        },
        None => Err(LinderaErrorKind::Args
            .with_error(anyhow::anyhow!("Invalid user dictionary source file"))),
    }
}
