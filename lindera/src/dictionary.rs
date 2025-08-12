use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[cfg(feature = "cc-cedict")]
use lindera_cc_cedict::DICTIONARY_NAME as CC_CEDICT_DICTIONARY_NAME;
#[cfg(all(feature = "cc-cedict", feature = "embedded-cc-cedict"))]
use lindera_cc_cedict::embedded::EmbeddedCcCedictLoader;
use lindera_dictionary::dictionary_builder::DictionaryBuilder;
use lindera_dictionary::dictionary_loader::DictionaryLoader;
use lindera_dictionary::dictionary_loader::StandardDictionaryLoader;
use lindera_dictionary::dictionary_loader::user_dictionary::UserDictionaryLoader;
#[cfg(feature = "ipadic")]
use lindera_ipadic::DICTIONARY_NAME as IPADIC_DICTIONARY_NAME;
#[cfg(all(feature = "ipadic", feature = "embedded-ipadic"))]
use lindera_ipadic::embedded::EmbeddedIPADICLoader;
#[cfg(feature = "ipadic-neologd")]
use lindera_ipadic_neologd::DICTIONARY_NAME as IPADIC_NEOLOGD_DICTIONARY_NAME;
#[cfg(all(feature = "ipadic-neologd", feature = "embedded-ipadic-neologd"))]
use lindera_ipadic_neologd::embedded::EmbeddedIPADICNEologdLoader;
#[cfg(feature = "ko-dic")]
use lindera_ko_dic::DICTIONARY_NAME as KO_DIC_DICTIONARY_NAME;
#[cfg(all(feature = "ko-dic", feature = "embedded-ko-dic"))]
use lindera_ko_dic::embedded::EmbeddedKoDicLoader;
#[cfg(feature = "unidic")]
use lindera_unidic::DICTIONARY_NAME as UNIDIC_DICTIONARY_NAME;
#[cfg(all(feature = "unidic", feature = "embedded-unidic"))]
use lindera_unidic::embedded::EmbeddedUniDicLoader;

use crate::LinderaResult;
use crate::error::{LinderaError, LinderaErrorKind};

pub type Dictionary = lindera_dictionary::dictionary::Dictionary;
pub type Metadata = lindera_dictionary::dictionary::metadata::Metadata;
pub type UserDictionary = lindera_dictionary::dictionary::UserDictionary;
pub type WordId = lindera_dictionary::viterbi::WordId;

#[derive(Debug, Clone, EnumIter, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryKind {
    #[cfg(feature = "ipadic")]
    #[serde(rename = "ipadic")]
    IPADIC,
    #[cfg(feature = "ipadic-neologd")]
    #[serde(rename = "ipadic-neologd")]
    IPADICNEologd,
    #[cfg(feature = "unidic")]
    #[serde(rename = "unidic")]
    UniDic,
    #[cfg(feature = "ko-dic")]
    #[serde(rename = "ko-dic")]
    KoDic,
    #[cfg(feature = "cc-cedict")]
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
                #[cfg(feature = "ipadic")]
                DictionaryKind::IPADIC => cfg!(feature = "ipadic"),
                #[cfg(feature = "ipadic-neologd")]
                DictionaryKind::IPADICNEologd => cfg!(feature = "ipadic-neologd"),
                #[cfg(feature = "unidic")]
                DictionaryKind::UniDic => cfg!(feature = "unidic"),
                #[cfg(feature = "ko-dic")]
                DictionaryKind::KoDic => cfg!(feature = "ko-dic"),
                #[cfg(feature = "cc-cedict")]
                DictionaryKind::CcCedict => cfg!(feature = "cc-cedict"),
                #[allow(unreachable_patterns)]
                _ => false,
            })
            .collect::<Vec<_>>()
    }

    pub fn as_str(&self) -> &str {
        match self {
            #[cfg(feature = "ipadic")]
            DictionaryKind::IPADIC => IPADIC_DICTIONARY_NAME,
            #[cfg(feature = "ipadic-neologd")]
            DictionaryKind::IPADICNEologd => IPADIC_NEOLOGD_DICTIONARY_NAME,
            #[cfg(feature = "unidic")]
            DictionaryKind::UniDic => UNIDIC_DICTIONARY_NAME,
            #[cfg(feature = "ko-dic")]
            DictionaryKind::KoDic => KO_DIC_DICTIONARY_NAME,
            #[cfg(feature = "cc-cedict")]
            DictionaryKind::CcCedict => CC_CEDICT_DICTIONARY_NAME,
            #[allow(unreachable_patterns)]
            _ => "",
        }
    }
}

impl FromStr for DictionaryKind {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryKind, Self::Err> {
        match input {
            #[cfg(feature = "ipadic")]
            IPADIC_DICTIONARY_NAME => Ok(DictionaryKind::IPADIC),
            #[cfg(feature = "ipadic-neologd")]
            IPADIC_NEOLOGD_DICTIONARY_NAME => Ok(DictionaryKind::IPADICNEologd),
            #[cfg(feature = "unidic")]
            UNIDIC_DICTIONARY_NAME => Ok(DictionaryKind::UniDic),
            #[cfg(feature = "ko-dic")]
            KO_DIC_DICTIONARY_NAME => Ok(DictionaryKind::KoDic),
            #[cfg(feature = "cc-cedict")]
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
        #[cfg(feature = "ipadic-neologd")]
        DictionaryKind::IPADICNEologd => Ok(DictionaryBuilder::new(
            lindera_ipadic_neologd::metadata::IPADICNEologdMetadata::metadata(),
        )),
        #[cfg(feature = "unidic")]
        DictionaryKind::UniDic => Ok(DictionaryBuilder::new(
            lindera_unidic::metadata::UniDicMetadata::metadata(),
        )),
        #[cfg(feature = "ko-dic")]
        DictionaryKind::KoDic => Ok(DictionaryBuilder::new(
            lindera_ko_dic::metadata::KoDicMetadata::metadata(),
        )),
        #[cfg(feature = "cc-cedict")]
        DictionaryKind::CcCedict => Ok(DictionaryBuilder::new(
            lindera_cc_cedict::metadata::CcCedictMetadata::metadata(),
        )),
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
        #[cfg(all(feature = "ipadic-neologd", feature = "embedded-ipadic-neologd"))]
        DictionaryKind::IPADICNEologd => Ok(Box::new(EmbeddedIPADICNEologdLoader::new())),
        #[cfg(all(feature = "ipadic-neologd", not(feature = "embedded-ipadic-neologd")))]
        DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::FeatureDisabled.with_error(
            anyhow::anyhow!("IPADIC-NEologd embedded feature is not enabled"),
        )),
        #[cfg(all(feature = "unidic", feature = "embedded-unidic"))]
        DictionaryKind::UniDic => Ok(Box::new(EmbeddedUniDicLoader::new())),
        #[cfg(all(feature = "unidic", not(feature = "embedded-unidic")))]
        DictionaryKind::UniDic => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("UniDic embedded feature is not enabled"))),
        #[cfg(all(feature = "ko-dic", feature = "embedded-ko-dic"))]
        DictionaryKind::KoDic => Ok(Box::new(EmbeddedKoDicLoader::new())),
        #[cfg(all(feature = "ko-dic", not(feature = "embedded-ko-dic")))]
        DictionaryKind::KoDic => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("KO-DIC embedded feature is not enabled"))),
        #[cfg(all(feature = "cc-cedict", feature = "embedded-cc-cedict"))]
        DictionaryKind::CcCedict => Ok(Box::new(EmbeddedCcCedictLoader::new())),
        #[cfg(all(feature = "cc-cedict", not(feature = "embedded-cc-cedict")))]
        DictionaryKind::CcCedict => Err(LinderaErrorKind::FeatureDisabled
            .with_error(anyhow::anyhow!("CC-CEDICT embedded feature is not enabled"))),
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
