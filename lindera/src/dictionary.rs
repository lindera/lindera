use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use lindera_dictionary::dictionary_builder::DictionaryBuilder;
use lindera_dictionary::dictionary_loader::user_dictionary::UserDictionaryLoader;
use lindera_dictionary::dictionary_loader::DictionaryLoader;
#[cfg(any(
    all(feature = "ipadic", not(feature = "embedded-ipadic")),
    all(feature = "ipadic-neologd", not(feature = "embedded-ipadic-neologd")),
    all(feature = "unidic", not(feature = "embedded-unidic")),
    all(feature = "ko-dic", not(feature = "embedded-ko-dic")),
    all(feature = "cc-cedict", not(feature = "embedded-cc-cedict"))
))]
use lindera_dictionary::dictionary_loader::StandardDictionaryLoader;

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
        DictionaryKind::IPADIC => Ok(DictionaryBuilder::new(
            lindera_ipadic::metadata::IpadicMetadata::metadata(),
        )),
        #[cfg(not(feature = "ipadic"))]
        DictionaryKind::IPADIC => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("IPADIC feature is not enabled"))),
        #[cfg(feature = "ipadic-neologd")]
        DictionaryKind::IPADICNEologd => Ok(DictionaryBuilder::new(
            lindera_ipadic_neologd::metadata::IpadicNeologdMetadata::metadata(),
        )),
        #[cfg(not(feature = "ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("IPADIC-NEologd feature is not enabled"))),
        #[cfg(feature = "unidic")]
        DictionaryKind::UniDic => Ok(DictionaryBuilder::new(
            lindera_unidic::metadata::UnidicMetadata::metadata(),
        )),
        #[cfg(not(feature = "unidic"))]
        DictionaryKind::UniDic => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("UniDic feature is not enabled"))),
        #[cfg(feature = "ko-dic")]
        DictionaryKind::KoDic => Ok(DictionaryBuilder::new(
            lindera_ko_dic::metadata::KoDicMetadata::metadata(),
        )),
        #[cfg(not(feature = "ko-dic"))]
        DictionaryKind::KoDic => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("KO-DIC feature is not enabled"))),
        #[cfg(feature = "cc-cedict")]
        DictionaryKind::CcCedict => Ok(DictionaryBuilder::new(
            lindera_cc_cedict::metadata::CcCedictMetadata::metadata(),
        )),
        #[cfg(not(feature = "cc-cedict"))]
        DictionaryKind::CcCedict => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("CC-CEDICT feature is not enabled"))),
    }
}

pub fn resolve_loader(dictionary_type: DictionaryKind) -> LinderaResult<Box<dyn DictionaryLoader>> {
    match dictionary_type {
        #[cfg(all(feature = "ipadic", feature = "embedded-ipadic"))]
        DictionaryKind::IPADIC => Ok(Box::new(lindera_ipadic::EmbeddedLoader)),
        #[cfg(all(feature = "ipadic", not(feature = "embedded-ipadic")))]
        DictionaryKind::IPADIC => Ok(Box::new(StandardDictionaryLoader::new(
            "IPADIC".to_string(),
            vec![
                "./dict/ipadic".to_string(),
                "./lindera-ipadic".to_string(),
                "/usr/local/share/lindera/ipadic".to_string(),
                "/usr/share/lindera/ipadic".to_string(),
            ],
            "LINDERA_IPADIC_PATH".to_string(),
        ))),
        #[cfg(not(feature = "ipadic"))]
        DictionaryKind::IPADIC => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("IPADIC feature is not enabled"))),
        #[cfg(all(feature = "ipadic-neologd", feature = "embedded-ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Ok(Box::new(lindera_ipadic_neologd::EmbeddedLoader)),
        #[cfg(all(feature = "ipadic-neologd", not(feature = "embedded-ipadic-neologd")))]
        DictionaryKind::IPADICNEologd => Ok(Box::new(StandardDictionaryLoader::new(
            "IPADIC-NEologd".to_string(),
            vec![
                "./dict/ipadic-neologd".to_string(),
                "./lindera-ipadic-neologd".to_string(),
                "/usr/local/share/lindera/ipadic-neologd".to_string(),
                "/usr/share/lindera/ipadic-neologd".to_string(),
            ],
            "LINDERA_IPADIC_NEOLOGD_PATH".to_string(),
        ))),
        #[cfg(not(feature = "ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("IPADIC-NEologd feature is not enabled"))),
        #[cfg(all(feature = "unidic", feature = "embedded-unidic"))]
        DictionaryKind::UniDic => Ok(Box::new(lindera_unidic::EmbeddedLoader)),
        #[cfg(all(feature = "unidic", not(feature = "embedded-unidic")))]
        DictionaryKind::UniDic => Ok(Box::new(StandardDictionaryLoader::new(
            "UniDic".to_string(),
            vec![
                "./dict/unidic".to_string(),
                "./lindera-unidic".to_string(),
                "/usr/local/share/lindera/unidic".to_string(),
                "/usr/share/lindera/unidic".to_string(),
            ],
            "LINDERA_UNIDIC_PATH".to_string(),
        ))),
        #[cfg(not(feature = "unidic"))]
        DictionaryKind::UniDic => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("UniDic feature is not enabled"))),
        #[cfg(all(feature = "ko-dic", feature = "embedded-ko-dic"))]
        DictionaryKind::KoDic => Ok(Box::new(lindera_ko_dic::EmbeddedLoader)),
        #[cfg(all(feature = "ko-dic", not(feature = "embedded-ko-dic")))]
        DictionaryKind::KoDic => Ok(Box::new(StandardDictionaryLoader::new(
            "Ko-Dic".to_string(),
            vec![
                "./dict/ko-dic".to_string(),
                "./lindera-ko-dic".to_string(),
                "/usr/local/share/lindera/ko-dic".to_string(),
                "/usr/share/lindera/ko-dic".to_string(),
            ],
            "LINDERA_KO_DIC_PATH".to_string(),
        ))),
        #[cfg(not(feature = "ko-dic"))]
        DictionaryKind::KoDic => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("KO-DIC feature is not enabled"))),
        #[cfg(all(feature = "cc-cedict", feature = "embedded-cc-cedict"))]
        DictionaryKind::CcCedict => Ok(Box::new(lindera_cc_cedict::EmbeddedLoader)),
        #[cfg(all(feature = "cc-cedict", not(feature = "embedded-cc-cedict")))]
        DictionaryKind::CcCedict => Ok(Box::new(StandardDictionaryLoader::new(
            "CC-CEDICT".to_string(),
            vec![
                "./dict/cc-cedict".to_string(),
                "./lindera-cc-cedict".to_string(),
                "/usr/local/share/lindera/cc-cedict".to_string(),
                "/usr/share/lindera/cc-cedict".to_string(),
            ],
            "LINDERA_CC_CEDICT_PATH".to_string(),
        ))),
        #[cfg(not(feature = "cc-cedict"))]
        DictionaryKind::CcCedict => Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("CC-CEDICT feature is not enabled"))),
    }
}

pub fn load_dictionary_from_path(path: &Path, use_mmap: bool) -> LinderaResult<Dictionary> {
    Dictionary::load_from_path_with_options(path, use_mmap)
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
    let builder = resolve_builder(kind)?;
    UserDictionaryLoader::load_from_csv(builder, path)
}

pub fn load_user_dictionary_from_bin(path: &Path) -> LinderaResult<UserDictionary> {
    UserDictionaryLoader::load_from_bin(path)
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
