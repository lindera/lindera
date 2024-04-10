use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use lindera_cc_cedict_builder::cc_cedict_builder::CcCedictBuilder;
use lindera_core::character_definition::CharacterDefinitions;
use lindera_core::connection::ConnectionCostMatrix;
use lindera_core::dictionary::{Dictionary, UserDictionary};
use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::error::{LinderaError, LinderaErrorKind};
use lindera_core::prefix_dict::PrefixDict;
use lindera_core::unknown_dictionary::UnknownDictionary;
use lindera_core::LinderaResult;
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;
use lindera_ipadic_neologd_builder::ipadic_neologd_builder::IpadicNeologdBuilder;
use lindera_ko_dic_builder::ko_dic_builder::KoDicBuilder;
use lindera_unidic_builder::unidic_builder::UnidicBuilder;

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

pub struct DictionaryBuilderResolver {}

impl DictionaryBuilderResolver {
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
}

pub struct DictionaryLoader {}

impl DictionaryLoader {
    fn read_file(path: PathBuf) -> LinderaResult<Vec<u8>> {
        fs::read(path).map_err(|e| LinderaErrorKind::Io.with_error(e))
    }

    pub fn prefix_dict(dir: PathBuf) -> LinderaResult<PrefixDict> {
        let unidic_data_path = dir.join("dict.da");
        let unidic_data = Self::read_file(unidic_data_path)?;

        let unidic_vals_path = dir.join("dict.vals");
        let unidic_vals = Self::read_file(unidic_vals_path)?;

        Ok(PrefixDict::from_static_slice(
            unidic_data.as_slice(),
            unidic_vals.as_slice(),
        ))
    }

    pub fn connection(dir: PathBuf) -> LinderaResult<ConnectionCostMatrix> {
        let path = dir.join("matrix.mtx");
        let data = Self::read_file(path)?;

        Ok(ConnectionCostMatrix::load(data.as_slice()))
    }

    pub fn char_def(dir: PathBuf) -> LinderaResult<CharacterDefinitions> {
        let path = dir.join("char_def.bin");
        let data = Self::read_file(path)?;

        CharacterDefinitions::load(data.as_slice())
    }

    pub fn unknown_dict(dir: PathBuf) -> LinderaResult<UnknownDictionary> {
        let path = dir.join("unk.bin");
        let data = Self::read_file(path)?;

        UnknownDictionary::load(data.as_slice())
    }

    pub fn words_idx_data(dir: PathBuf) -> LinderaResult<Vec<u8>> {
        let path = dir.join("dict.wordsidx");
        Self::read_file(path)
    }

    pub fn words_data(dir: PathBuf) -> LinderaResult<Vec<u8>> {
        let path = dir.join("dict.words");
        Self::read_file(path)
    }

    pub fn load_dictionary(path: PathBuf) -> LinderaResult<Dictionary> {
        Ok(Dictionary {
            dict: Self::prefix_dict(path.clone())?,
            cost_matrix: Self::connection(path.clone())?,
            char_definitions: Self::char_def(path.clone())?,
            unknown_dictionary: Self::unknown_dict(path.clone())?,
            words_idx_data: Cow::Owned(Self::words_idx_data(path.clone())?),
            words_data: Cow::Owned(Self::words_data(path)?),
        })
    }

    pub fn load_dictionary_from_kind(kind: DictionaryKind) -> LinderaResult<Dictionary> {
        // The dictionary specified by the feature flag will be loaded.
        match kind {
            #[cfg(feature = "ipadic")]
            DictionaryKind::IPADIC => lindera_ipadic::load_dictionary()
                .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e)),
            #[cfg(feature = "ipadic-neologd")]
            DictionaryKind::IPADICNEologd => lindera_ipadic_neologd::load_dictionary()
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
                Self::load_dictionary_from_kind(kind)
            }
            None => {
                match dictionary_config.path {
                    Some(path) => {
                        // load external dictionary from path
                        Self::load_dictionary(path)
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
        let builder = DictionaryBuilderResolver::resolve_builder(kind)?;
        builder
            .build_user_dict(path.as_path())
            .map_err(|err| LinderaErrorKind::DictionaryBuildError.with_error(err))
    }

    pub fn load_user_dictionary_from_bin(path: PathBuf) -> LinderaResult<UserDictionary> {
        UserDictionary::load(&Self::read_file(path)?)
    }

    pub fn load_user_dictionary_from_config(
        dictionary_config: UserDictionaryConfig,
    ) -> LinderaResult<UserDictionary> {
        match dictionary_config.path.extension() {
            Some(ext) => match ext.to_str() {
                Some("csv") => match dictionary_config.kind {
                    Some(kind) => Self::load_user_dictionary_from_csv(kind, dictionary_config.path),
                    None => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                        "Dictionary type must be specified if CSV file specified"
                    ))),
                },
                Some("bin") => Self::load_user_dictionary_from_bin(dictionary_config.path),
                _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                    "Invalid user dictionary source file extension"
                ))),
            },
            None => Err(LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("Invalid user dictionary source file"))),
        }
    }
}
