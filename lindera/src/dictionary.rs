use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[cfg(all(feature = "cc-cedict", feature = "embedded-cc-cedict"))]
use lindera_cc_cedict::embedded::EmbeddedCcCedictLoader;
use lindera_cc_cedict::metadata::DICTIONARY_NAME as CC_CEDICT_DICTIONARY_NAME;
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
use lindera_dictionary::dictionary_loader::DictionaryLoader;
use lindera_dictionary::dictionary_loader::StandardDictionaryLoader;
use lindera_dictionary::dictionary_loader::user_dictionary::UserDictionaryLoader;
#[cfg(all(feature = "ipadic", feature = "embedded-ipadic"))]
use lindera_ipadic::embedded::EmbeddedIPADICLoader;
use lindera_ipadic::metadata::DICTIONARY_NAME as IPADIC_DICTIONARY_NAME;
#[cfg(all(feature = "ipadic-neologd", feature = "embedded-ipadic-neologd"))]
use lindera_ipadic_neologd::embedded::EmbeddedIPADICNEologdLoader;
use lindera_ipadic_neologd::metadata::DICTIONARY_NAME as IPADIC_NEOLOGD_DICTIONARY_NAME;
#[cfg(all(feature = "ko-dic", feature = "embedded-ko-dic"))]
use lindera_ko_dic::embedded::EmbeddedKoDicLoader;
use lindera_ko_dic::metadata::DICTIONARY_NAME as KO_DIC_DICTIONARY_NAME;
#[cfg(all(feature = "unidic", feature = "embedded-unidic"))]
use lindera_unidic::embedded::EmbeddedUniDicLoader;
use lindera_unidic::metadata::DICTIONARY_NAME as UNIDIC_DICTIONARY_NAME;

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
            DictionaryKind::IPADIC => IPADIC_DICTIONARY_NAME,
            DictionaryKind::IPADICNEologd => IPADIC_NEOLOGD_DICTIONARY_NAME,
            DictionaryKind::UniDic => UNIDIC_DICTIONARY_NAME,
            DictionaryKind::KoDic => KO_DIC_DICTIONARY_NAME,
            DictionaryKind::CcCedict => CC_CEDICT_DICTIONARY_NAME,
        }
    }
}

impl FromStr for DictionaryKind {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryKind, Self::Err> {
        match input {
            IPADIC_DICTIONARY_NAME => Ok(DictionaryKind::IPADIC),
            IPADIC_NEOLOGD_DICTIONARY_NAME => Ok(DictionaryKind::IPADICNEologd),
            UNIDIC_DICTIONARY_NAME => Ok(DictionaryKind::UniDic),
            KO_DIC_DICTIONARY_NAME => Ok(DictionaryKind::KoDic),
            CC_CEDICT_DICTIONARY_NAME => Ok(DictionaryKind::CcCedict),
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
            lindera_ipadic::metadata::IPADICMetadata::metadata(),
        )),
        #[cfg(not(feature = "ipadic"))]
        DictionaryKind::IPADIC => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("IPADIC feature is not enabled"))),
        #[cfg(feature = "ipadic-neologd")]
        DictionaryKind::IPADICNEologd => Ok(DictionaryBuilder::new(
            lindera_ipadic_neologd::metadata::IPADICNEologdMetadata::metadata(),
        )),
        #[cfg(not(feature = "ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("IPADIC-NEologd feature is not enabled"))),
        #[cfg(feature = "unidic")]
        DictionaryKind::UniDic => Ok(DictionaryBuilder::new(
            lindera_unidic::metadata::UniDicMetadata::metadata(),
        )),
        #[cfg(not(feature = "unidic"))]
        DictionaryKind::UniDic => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("UniDic feature is not enabled"))),
        #[cfg(feature = "ko-dic")]
        DictionaryKind::KoDic => Ok(DictionaryBuilder::new(
            lindera_ko_dic::metadata::KoDicMetadata::metadata(),
        )),
        #[cfg(not(feature = "ko-dic"))]
        DictionaryKind::KoDic => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("KO-DIC feature is not enabled"))),
        #[cfg(feature = "cc-cedict")]
        DictionaryKind::CcCedict => Ok(DictionaryBuilder::new(
            lindera_cc_cedict::metadata::CcCedictMetadata::metadata(),
        )),
        #[cfg(not(feature = "cc-cedict"))]
        DictionaryKind::CcCedict => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("CC-CEDICT feature is not enabled"))),
    }
}

pub fn resolve_embedded_loader(
    dictionary_type: DictionaryKind,
) -> LinderaResult<Box<dyn DictionaryLoader>> {
    match dictionary_type {
        #[cfg(all(feature = "ipadic", feature = "embedded-ipadic"))]
        DictionaryKind::IPADIC => Ok(Box::new(EmbeddedIPADICLoader::new())),
        #[cfg(all(feature = "ipadic", not(feature = "embedded-ipadic")))]
        DictionaryKind::IPADIC => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("IPADIC embedded feature is not enabled"))),
        #[cfg(not(feature = "ipadic"))]
        DictionaryKind::IPADIC => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("IPADIC feature is not enabled"))),
        #[cfg(all(feature = "ipadic-neologd", feature = "embedded-ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Ok(Box::new(EmbeddedIPADICNEologdLoader::new())),
        #[cfg(all(feature = "ipadic-neologd", not(feature = "embedded-ipadic-neologd")))]
        DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::FeatureDisabled.with_error(
            anyhow::anyhow!("IPADIC-NEologd embedded feature is not enabled"),
        )),
        #[cfg(not(feature = "ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("IPADIC-NEologd feature is not enabled"))),
        #[cfg(all(feature = "unidic", feature = "embedded-unidic"))]
        DictionaryKind::UniDic => Ok(Box::new(EmbeddedUniDicLoader::new())),
        #[cfg(all(feature = "unidic", not(feature = "embedded-unidic")))]
        DictionaryKind::UniDic => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("UniDic embedded feature is not enabled"))),
        #[cfg(not(feature = "unidic"))]
        DictionaryKind::UniDic => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("UniDic feature is not enabled"))),
        #[cfg(all(feature = "ko-dic", feature = "embedded-ko-dic"))]
        DictionaryKind::KoDic => Ok(Box::new(EmbeddedKoDicLoader::new())),
        #[cfg(all(feature = "ko-dic", not(feature = "embedded-ko-dic")))]
        DictionaryKind::KoDic => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("KO-DIC embedded feature is not enabled"))),
        #[cfg(not(feature = "ko-dic"))]
        DictionaryKind::KoDic => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("KO-DIC feature is not enabled"))),
        #[cfg(all(feature = "cc-cedict", feature = "embedded-cc-cedict"))]
        DictionaryKind::CcCedict => Ok(Box::new(EmbeddedCcCedictLoader::new())),
        #[cfg(all(feature = "cc-cedict", not(feature = "embedded-cc-cedict")))]
        DictionaryKind::CcCedict => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("CC-CEDICT embedded feature is not enabled"))),
        #[cfg(not(feature = "cc-cedict"))]
        DictionaryKind::CcCedict => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("CC-CEDICT feature is not enabled"))),
    }
}

pub fn load_dictionary(path: &Path) -> LinderaResult<Dictionary> {
    let loader = StandardDictionaryLoader::new();
    loader.load_from_path(path)
}

pub fn load_embedded_dictionary(kind: DictionaryKind) -> LinderaResult<Dictionary> {
    let loader = resolve_embedded_loader(kind)?;
    loader
        .load()
        .map_err(|e| LinderaErrorKind::NotFound.with_error(e))
}

pub fn load_dictionary_from_config(
    dictionary_config: &DictionaryConfig,
) -> LinderaResult<Dictionary> {
    // Try loading embedded dictionary first
    if let Some(kind_value) = dictionary_config.get("kind") {
        let kind_str = kind_value.as_str().ok_or_else(|| {
            LinderaErrorKind::Parse.with_error(anyhow::anyhow!("kind field must be a string"))
        })?;

        let kind = DictionaryKind::from_str(kind_str)?;
        return load_embedded_dictionary(kind);
    }

    // Otherwise, try loading from path
    if let Some(path_value) = dictionary_config.get("path") {
        let path_str = path_value.as_str().ok_or_else(|| {
            LinderaErrorKind::Parse.with_error(anyhow::anyhow!("path field must be a string"))
        })?;

        let path = PathBuf::from(path_str);
        return load_dictionary(&path);
    }

    Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
        "kind field or path field must be specified"
    )))
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

fn get_path_from_config(dictionary_config: &UserDictionaryConfig) -> LinderaResult<PathBuf> {
    let path_value = dictionary_config.get("path").ok_or_else(|| {
        LinderaErrorKind::Args.with_error(anyhow::anyhow!(
            "path field must be specified in user dictionary config"
        ))
    })?;

    let path_str = path_value.as_str().ok_or_else(|| {
        LinderaErrorKind::Parse.with_error(anyhow::anyhow!("path field must be a string"))
    })?;

    Ok(PathBuf::from(path_str))
}

fn get_kind_from_config(dictionary_config: &UserDictionaryConfig) -> LinderaResult<DictionaryKind> {
    let kind_value = dictionary_config.get("kind").ok_or_else(|| {
        LinderaErrorKind::Args.with_error(anyhow::anyhow!(
            "kind field must be specified if CSV file specified"
        ))
    })?;

    let kind_str = kind_value.as_str().ok_or_else(|| {
        LinderaErrorKind::Parse.with_error(anyhow::anyhow!("kind field must be a string"))
    })?;

    DictionaryKind::from_str(kind_str)
}

pub fn load_user_dictionary_from_config(
    dictionary_config: &UserDictionaryConfig,
) -> LinderaResult<UserDictionary> {
    let path = get_path_from_config(dictionary_config)?;

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("Invalid user dictionary source file"))
        })?;

    match extension {
        "csv" => {
            let kind = get_kind_from_config(dictionary_config)?;
            load_user_dictionary_from_csv(kind, &path)
        }
        "bin" => load_user_dictionary_from_bin(&path),
        _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
            "Invalid user dictionary source file extension"
        ))),
    }
}
