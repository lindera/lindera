use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use lindera_dictionary::dictionary_builder::cc_cedict::CcCedictBuilder;
use lindera_dictionary::dictionary_builder::ipadic::IpadicBuilder;
use lindera_dictionary::dictionary_builder::ipadic_neologd::IpadicNeologdBuilder;
use lindera_dictionary::dictionary_builder::ko_dic::KoDicBuilder;
use lindera_dictionary::dictionary_builder::unidic::UnidicBuilder;
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
use lindera_dictionary::dictionary_loader::character_definition::CharacterDefinitionLoader;
use lindera_dictionary::dictionary_loader::connection_cost_matrix::ConnectionCostMatrixLoader;
use lindera_dictionary::dictionary_loader::prefix_dictionary::PrefixDictionaryLoader;
use lindera_dictionary::dictionary_loader::unknown_dictionary::UnknownDictionaryLoader;
use lindera_dictionary::util::read_file;

use crate::error::{LinderaError, LinderaErrorKind};
use crate::LinderaResult;

pub type Dictionary = lindera_dictionary::dictionary::Dictionary;
pub type UserDictionary = lindera_dictionary::dictionary::UserDictionary;
pub type WordId = lindera_dictionary::viterbi::WordId;

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

pub fn load_dictionary_from_path(path: &Path) -> LinderaResult<Dictionary> {
    Ok(Dictionary {
        prefix_dictionary: PrefixDictionaryLoader::load(path)?,
        connection_cost_matrix: ConnectionCostMatrixLoader::load(path)?,
        character_definition: CharacterDefinitionLoader::load(path)?,
        unknown_dictionary: UnknownDictionaryLoader::load(path)?,
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
                    load_dictionary_from_path(path.as_path())
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
