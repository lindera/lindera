use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use lindera_dictionary::dictionary_builder::DictionaryBuilder;
use lindera_dictionary::dictionary_loader::DictionaryLoader;
use lindera_dictionary::dictionary_loader::character_definition::CharacterDefinitionLoader;
use lindera_dictionary::dictionary_loader::connection_cost_matrix::ConnectionCostMatrixLoader;
use lindera_dictionary::dictionary_loader::metadata::MetadataLoader;
use lindera_dictionary::dictionary_loader::prefix_dictionary::PrefixDictionaryLoader;
use lindera_dictionary::dictionary_loader::unknown_dictionary::UnknownDictionaryLoader;
use lindera_dictionary::util::read_file;

use crate::LinderaResult;
use crate::error::{LinderaError, LinderaErrorKind};

pub type Dictionary = lindera_dictionary::dictionary::Dictionary;
pub type Metadata = lindera_dictionary::dictionary::metadata::Metadata;
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
            _ => Err(LinderaErrorKind::Dictionary
                .with_error(anyhow::anyhow!("Invalid dictionary kind: {}", input))),
        }
    }
}

pub type DictionaryConfig = Value;
pub type UserDictionaryConfig = Value;

pub fn resolve_builder(dictionary_type: DictionaryKind) -> LinderaResult<DictionaryBuilder> {
    match dictionary_type {
        #[cfg(feature = "ipadic")]
        DictionaryKind::IPADIC => Ok(lindera_ipadic::create_builder()),
        #[cfg(not(feature = "ipadic"))]
        DictionaryKind::IPADIC => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("IPADIC feature is not enabled"))),
        #[cfg(feature = "ipadic-neologd")]
        DictionaryKind::IPADICNEologd => Ok(lindera_ipadic_neologd::create_builder()),
        #[cfg(not(feature = "ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("IPADIC-NEologd feature is not enabled"))),
        #[cfg(feature = "unidic")]
        DictionaryKind::UniDic => Ok(lindera_unidic::create_builder()),
        #[cfg(not(feature = "unidic"))]
        DictionaryKind::UniDic => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("UniDic feature is not enabled"))),
        #[cfg(feature = "ko-dic")]
        DictionaryKind::KoDic => Ok(lindera_ko_dic::create_builder()),
        #[cfg(not(feature = "ko-dic"))]
        DictionaryKind::KoDic => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("KO-DIC feature is not enabled"))),
        #[cfg(feature = "cc-cedict")]
        DictionaryKind::CcCedict => Ok(lindera_cc_cedict::create_builder()),
        #[cfg(not(feature = "cc-cedict"))]
        DictionaryKind::CcCedict => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("CC-CEDICT feature is not enabled"))),
    }
}

pub fn resolve_loader(dictionary_type: DictionaryKind) -> LinderaResult<Box<dyn DictionaryLoader>> {
    match dictionary_type {
        #[cfg(feature = "ipadic")]
        DictionaryKind::IPADIC => Ok(Box::new(lindera_ipadic::create_loader())),
        #[cfg(not(feature = "ipadic"))]
        DictionaryKind::IPADIC => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("IPADIC feature is not enabled"))),
        #[cfg(feature = "ipadic-neologd")]
        DictionaryKind::IPADICNEologd => Ok(Box::new(lindera_ipadic_neologd::create_loader())),
        #[cfg(not(feature = "ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("IPADIC-NEologd feature is not enabled"))),
        #[cfg(feature = "unidic")]
        DictionaryKind::UniDic => Ok(Box::new(lindera_unidic::create_loader())),
        #[cfg(not(feature = "unidic"))]
        DictionaryKind::UniDic => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("UniDic feature is not enabled"))),
        #[cfg(feature = "ko-dic")]
        DictionaryKind::KoDic => Ok(Box::new(lindera_ko_dic::create_loader())),
        #[cfg(not(feature = "ko-dic"))]
        DictionaryKind::KoDic => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("KO-DIC feature is not enabled"))),
        #[cfg(feature = "cc-cedict")]
        DictionaryKind::CcCedict => Ok(Box::new(lindera_cc_cedict::create_loader())),
        #[cfg(not(feature = "cc-cedict"))]
        DictionaryKind::CcCedict => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("CC-CEDICT feature is not enabled"))),
    }
}

pub fn load_dictionary_from_path(path: &Path, use_mmap: bool) -> LinderaResult<Dictionary> {
    Ok(Dictionary {
        prefix_dictionary: {
            #[cfg(feature = "mmap")]
            if use_mmap {
                PrefixDictionaryLoader::load_mmap(path)?
            } else {
                PrefixDictionaryLoader::load(path)?
            }
            #[cfg(not(feature = "mmap"))]
            PrefixDictionaryLoader::load(path)?
        },
        connection_cost_matrix: {
            #[cfg(feature = "mmap")]
            if use_mmap {
                ConnectionCostMatrixLoader::load_mmap(path)?
            } else {
                ConnectionCostMatrixLoader::load(path)?
            }
            #[cfg(not(feature = "mmap"))]
            ConnectionCostMatrixLoader::load(path)?
        },
        character_definition: CharacterDefinitionLoader::load(path)?,
        unknown_dictionary: UnknownDictionaryLoader::load(path)?,
        metadata: MetadataLoader::load(path)?, // Metadata is small, so normal loading is sufficient
    })
}

pub fn load_dictionary_from_kind(kind: DictionaryKind) -> LinderaResult<Dictionary> {
    let loader = resolve_loader(kind)?;
    loader
        .load()
        .map_err(|e| LinderaErrorKind::NotFound.with_error(e))
}

pub fn load_dictionary_from_config(
    dictionary_config: &DictionaryConfig,
) -> LinderaResult<Dictionary> {
    match dictionary_config.get("kind") {
        Some(kind_value) => {
            let kind = DictionaryKind::from_str(kind_value.as_str().ok_or_else(|| {
                LinderaErrorKind::Parse.with_error(anyhow::anyhow!("kind field must be a string"))
            })?)?;
            // Load contained dictionary from kind value in config.
            load_dictionary_from_kind(kind)
        }
        None => {
            match dictionary_config.get("path") {
                Some(path_value) => {
                    let path = PathBuf::from(path_value.as_str().ok_or_else(|| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("path field must be a string"))
                    })?);

                    // load external dictionary from path
                    // Use mmap by default if feature is enabled, unless explicitly disabled
                    let use_mmap = cfg!(feature = "mmap")
                        && !dictionary_config
                            .get("disable_mmap")
                            .is_some_and(|x| x.as_bool().is_some_and(|b| b));

                    load_dictionary_from_path(path.as_path(), use_mmap)
                }
                None => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                    "kind field or path field must be specified"
                ))),
            }
        }
    }
}

pub fn load_user_dictionary_from_csv(
    kind: DictionaryKind,
    path: &Path,
) -> LinderaResult<UserDictionary> {
    // Resolve the builder for the specified dictionary kind.
    let builder = resolve_builder(kind)?;
    builder.build_user_dict(path)
}

pub fn load_user_dictionary_from_bin(path: &Path) -> LinderaResult<UserDictionary> {
    UserDictionary::load(&read_file(path)?)
}

pub fn load_user_dictionary_from_config(
    dictionary_config: &UserDictionaryConfig,
) -> LinderaResult<UserDictionary> {
    match dictionary_config.get("path") {
        Some(path_value) => {
            let path = PathBuf::from(path_value.as_str().ok_or_else(|| {
                LinderaErrorKind::Parse.with_error(anyhow::anyhow!("path field must be a string"))
            })?);

            match path.extension() {
                Some(ext) => match ext.to_str() {
                    Some("csv") => match dictionary_config.get("kind") {
                        Some(kind_value) => {
                            let kind = DictionaryKind::from_str(kind_value.as_str().ok_or_else(
                                || {
                                    LinderaErrorKind::Parse
                                        .with_error(anyhow::anyhow!("kind field must be a string"))
                                },
                            )?)?;
                            load_user_dictionary_from_csv(kind, path.as_path())
                        }
                        None => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                            "kind field must be specified if CSV file specified"
                        ))),
                    },
                    Some("bin") => load_user_dictionary_from_bin(path.as_path()),
                    _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                        "Invalid user dictionary source file extension"
                    ))),
                },
                None => Err(LinderaErrorKind::Args
                    .with_error(anyhow::anyhow!("Invalid user dictionary source file"))),
            }
        }
        None => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
            "path field must be specified in user dictionary config"
        ))),
    }
}
