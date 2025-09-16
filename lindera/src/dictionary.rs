use std::path::{Path, PathBuf};
use std::str::FromStr;

use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use url::Url;

#[cfg(feature = "embedded-cc-cedict")]
use lindera_cc_cedict::DICTIONARY_NAME as CC_CEDICT_DICTIONARY_NAME;
#[cfg(feature = "embedded-cc-cedict")]
use lindera_cc_cedict::embedded::EmbeddedCcCedictLoader;
use lindera_dictionary::dictionary_loader::DictionaryLoader;
use lindera_dictionary::dictionary_loader::FSDictionaryLoader;
use lindera_dictionary::dictionary_loader::user_dictionary::UserDictionaryLoader;

#[cfg(feature = "train")]
pub use lindera_dictionary::trainer;
#[cfg(feature = "embedded-ipadic")]
use lindera_ipadic::DICTIONARY_NAME as IPADIC_DICTIONARY_NAME;
#[cfg(feature = "embedded-ipadic")]
use lindera_ipadic::embedded::EmbeddedIPADICLoader;
#[cfg(feature = "embedded-ipadic-neologd")]
use lindera_ipadic_neologd::DICTIONARY_NAME as IPADIC_NEOLOGD_DICTIONARY_NAME;
#[cfg(feature = "embedded-ipadic-neologd")]
use lindera_ipadic_neologd::embedded::EmbeddedIPADICNEologdLoader;
#[cfg(feature = "embedded-ko-dic")]
use lindera_ko_dic::DICTIONARY_NAME as KO_DIC_DICTIONARY_NAME;
#[cfg(feature = "embedded-ko-dic")]
use lindera_ko_dic::embedded::EmbeddedKoDicLoader;
#[cfg(feature = "embedded-unidic")]
use lindera_unidic::DICTIONARY_NAME as UNIDIC_DICTIONARY_NAME;
#[cfg(feature = "embedded-unidic")]
use lindera_unidic::embedded::EmbeddedUniDicLoader;

use crate::LinderaResult;
use crate::error::{LinderaError, LinderaErrorKind};

pub type Dictionary = lindera_dictionary::dictionary::Dictionary;
pub type Metadata = lindera_dictionary::dictionary::metadata::Metadata;
pub type UserDictionary = lindera_dictionary::dictionary::UserDictionary;
pub type WordId = lindera_dictionary::viterbi::WordId;
pub type DictionaryBuilder = lindera_dictionary::dictionary_builder::DictionaryBuilder;
pub type DictionaryConfig = Value;
pub type UserDictionaryConfig = Value;
pub type Schema = lindera_dictionary::dictionary::schema::Schema;
pub type FieldDefinition = lindera_dictionary::dictionary::schema::FieldDefinition;
pub type FieldType = lindera_dictionary::dictionary::schema::FieldType;
pub type CompressionAlgorithm = lindera_dictionary::decompress::Algorithm;

#[derive(Debug, Clone, EnumIter, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryScheme {
    #[cfg(any(
        feature = "embedded-ipadic",
        feature = "embedded-ipadic-neologd",
        feature = "embedded-unidic",
        feature = "embedded-ko-dic",
        feature = "embedded-cc-cedict",
    ))]
    #[serde(rename = "embedded")]
    Embedded,
    #[serde(rename = "file")]
    File,
}

impl DictionaryScheme {
    pub fn as_str(&self) -> &str {
        match self {
            #[cfg(any(
                feature = "embedded-ipadic",
                feature = "embedded-ipadic-neologd",
                feature = "embedded-unidic",
                feature = "embedded-ko-dic",
                feature = "embedded-cc-cedict",
            ))]
            DictionaryScheme::Embedded => "embedded",
            DictionaryScheme::File => "file",
        }
    }
}

impl FromStr for DictionaryScheme {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryScheme, Self::Err> {
        match input {
            #[cfg(any(
                feature = "embedded-ipadic",
                feature = "embedded-ipadic-neologd",
                feature = "embedded-unidic",
                feature = "embedded-ko-dic",
                feature = "embedded-cc-cedict",
            ))]
            "embedded" => Ok(DictionaryScheme::Embedded),
            "file" => Ok(DictionaryScheme::File),
            _ => Err(LinderaErrorKind::Dictionary
                .with_error(anyhow::anyhow!("Invalid dictionary scheme: {}", input))),
        }
    }
}

#[derive(Debug, Clone, EnumIter, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryKind {
    #[cfg(feature = "embedded-ipadic")]
    #[serde(rename = "ipadic")]
    IPADIC,
    #[cfg(feature = "embedded-ipadic-neologd")]
    #[serde(rename = "ipadic-neologd")]
    IPADICNEologd,
    #[cfg(feature = "embedded-unidic")]
    #[serde(rename = "unidic")]
    UniDic,
    #[cfg(feature = "embedded-ko-dic")]
    #[serde(rename = "ko-dic")]
    KoDic,
    #[cfg(feature = "embedded-cc-cedict")]
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
                #[cfg(feature = "embedded-ipadic")]
                DictionaryKind::IPADIC => cfg!(feature = "embedded-ipadic"),
                #[cfg(feature = "embedded-ipadic-neologd")]
                DictionaryKind::IPADICNEologd => cfg!(feature = "embedded-ipadic-neologd"),
                #[cfg(feature = "embedded-unidic")]
                DictionaryKind::UniDic => cfg!(feature = "embedded-unidic"),
                #[cfg(feature = "embedded-ko-dic")]
                DictionaryKind::KoDic => cfg!(feature = "embedded-ko-dic"),
                #[cfg(feature = "embedded-cc-cedict")]
                DictionaryKind::CcCedict => cfg!(feature = "embedded-cc-cedict"),
                #[allow(unreachable_patterns)]
                _ => false,
            })
            .collect::<Vec<_>>()
    }

    pub fn as_str(&self) -> &str {
        match self {
            #[cfg(feature = "embedded-ipadic")]
            DictionaryKind::IPADIC => IPADIC_DICTIONARY_NAME,
            #[cfg(feature = "embedded-ipadic-neologd")]
            DictionaryKind::IPADICNEologd => IPADIC_NEOLOGD_DICTIONARY_NAME,
            #[cfg(feature = "embedded-unidic")]
            DictionaryKind::UniDic => UNIDIC_DICTIONARY_NAME,
            #[cfg(feature = "embedded-ko-dic")]
            DictionaryKind::KoDic => KO_DIC_DICTIONARY_NAME,
            #[cfg(feature = "embedded-cc-cedict")]
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
            #[cfg(feature = "embedded-ipadic")]
            IPADIC_DICTIONARY_NAME => Ok(DictionaryKind::IPADIC),
            #[cfg(feature = "embedded-ipadic-neologd")]
            IPADIC_NEOLOGD_DICTIONARY_NAME => Ok(DictionaryKind::IPADICNEologd),
            #[cfg(feature = "embedded-unidic")]
            UNIDIC_DICTIONARY_NAME => Ok(DictionaryKind::UniDic),
            #[cfg(feature = "embedded-ko-dic")]
            KO_DIC_DICTIONARY_NAME => Ok(DictionaryKind::KoDic),
            #[cfg(feature = "embedded-cc-cedict")]
            CC_CEDICT_DICTIONARY_NAME => Ok(DictionaryKind::CcCedict),
            _ => Err(LinderaErrorKind::Dictionary
                .with_error(anyhow::anyhow!("Invalid dictionary kind: {}", input))),
        }
    }
}

pub fn resolve_embedded_loader(
    dictionary_type: DictionaryKind,
) -> LinderaResult<Box<dyn DictionaryLoader>> {
    match dictionary_type {
        #[cfg(feature = "embedded-ipadic")]
        DictionaryKind::IPADIC => Ok(Box::new(EmbeddedIPADICLoader::new())),
        // #[cfg(not(feature = "embedded-ipadic"))]
        // DictionaryKind::IPADIC => Err(LinderaErrorKind::FeatureDisabled
        //     .with_error(anyhow::anyhow!("IPADIC embedded feature is not enabled"))),
        #[cfg(feature = "embedded-ipadic-neologd")]
        DictionaryKind::IPADICNEologd => Ok(Box::new(EmbeddedIPADICNEologdLoader::new())),
        // #[cfg(not(feature = "embedded-ipadic-neologd"))]
        // DictionaryKind::IPADICNEologd => Err(LinderaErrorKind::FeatureDisabled.with_error(
        //     anyhow::anyhow!("IPADIC-NEologd embedded feature is not enabled"),
        // )),
        #[cfg(feature = "embedded-unidic")]
        DictionaryKind::UniDic => Ok(Box::new(EmbeddedUniDicLoader::new())),
        // #[cfg(not(feature = "embedded-unidic"))]
        // DictionaryKind::UniDic => Err(LinderaErrorKind::FeatureDisabled
        //     .with_error(anyhow::anyhow!("UniDic embedded feature is not enabled"))),
        #[cfg(feature = "embedded-ko-dic")]
        DictionaryKind::KoDic => Ok(Box::new(EmbeddedKoDicLoader::new())),
        // #[cfg(not(feature = "embedded-ko-dic"))]
        // DictionaryKind::KoDic => Err(LinderaErrorKind::FeatureDisabled
        //     .with_error(anyhow::anyhow!("KO-DIC embedded feature is not enabled"))),
        #[cfg(feature = "embedded-cc-cedict")]
        DictionaryKind::CcCedict => Ok(Box::new(EmbeddedCcCedictLoader::new())),
        // #[cfg(not(feature = "embedded-cc-cedict"))]
        // DictionaryKind::CcCedict => Err(LinderaErrorKind::FeatureDisabled
        //     .with_error(anyhow::anyhow!("CC-CEDICT embedded feature is not enabled"))),
    }
}

pub fn load_fs_dictionary(path: &Path) -> LinderaResult<Dictionary> {
    let loader = FSDictionaryLoader::new();
    loader.load_from_path(path)
}

pub fn load_embedded_dictionary(kind: DictionaryKind) -> LinderaResult<Dictionary> {
    let loader = resolve_embedded_loader(kind)?;
    loader
        .load()
        .map_err(|e| LinderaErrorKind::NotFound.with_error(e))
}

pub fn load_dictionary(uri: &str) -> LinderaResult<Dictionary> {
    // Try to parse as URI first, but only if it looks like a URI
    // (contains "://" or starts with known schemes)
    if uri.contains("://") {
        match Url::parse(uri) {
            Ok(parsed_uri) => {
                // Parse the URI and return the appropriate dictionary
                let scheme = DictionaryScheme::from_str(parsed_uri.scheme()).map_err(|err| {
                    LinderaErrorKind::Dictionary
                        .with_error(anyhow::anyhow!("Invalid dictionary scheme: {}", err))
                })?;

                match scheme {
                    #[cfg(any(
                        feature = "embedded-ipadic",
                        feature = "embedded-ipadic-neologd",
                        feature = "embedded-unidic",
                        feature = "embedded-ko-dic",
                        feature = "embedded-cc-cedict",
                    ))]
                    DictionaryScheme::Embedded => {
                        let kind = DictionaryKind::from_str(parsed_uri.host_str().unwrap_or(""))
                            .map_err(|err| LinderaErrorKind::Dictionary.with_error(err))?;

                        // Load the embedded dictionary
                        load_embedded_dictionary(kind)
                    }
                    DictionaryScheme::File => {
                        // Extract path from file:// URL manually
                        let path_str = parsed_uri.path();

                        // Handle Windows paths that might start with /C:/ etc.
                        let path_str =
                            if cfg!(windows) && path_str.len() > 1 && path_str.starts_with('/') {
                                &path_str[1..]
                            } else {
                                path_str
                            };

                        // Decode percent-encoded characters
                        let decoded_path =
                            percent_decode_str(path_str).decode_utf8().map_err(|e| {
                                LinderaErrorKind::Dictionary
                                    .with_error(anyhow::anyhow!("Invalid UTF-8 in path: {}", e))
                            })?;

                        let path = Path::new(decoded_path.as_ref());

                        // Load the file-based dictionary
                        load_fs_dictionary(path)
                    }
                }
            }
            Err(e) => Err(LinderaErrorKind::Dictionary
                .with_error(anyhow::anyhow!("Invalid URI format: {}", e))),
        }
    } else {
        // Treat it as a file path directly
        let path = Path::new(uri);
        load_fs_dictionary(path)
    }
}

pub fn load_user_dictionary_from_csv(
    metadata: &Metadata,
    path: &Path,
) -> LinderaResult<UserDictionary> {
    let builder = DictionaryBuilder::new(metadata.clone());
    UserDictionaryLoader::load_from_csv(builder, path)
}

pub fn load_user_dictionary_from_bin(path: &Path) -> LinderaResult<UserDictionary> {
    UserDictionaryLoader::load_from_bin(path)
}

pub fn load_user_dictionary(uri: &str, metadata: &Metadata) -> LinderaResult<UserDictionary> {
    // Try to parse as URI first, but only if it looks like a URI
    // (contains "://" or starts with known schemes)
    let path = if uri.contains("://") {
        match Url::parse(uri) {
            Ok(parsed_uri) => {
                // Parse the URI and return the appropriate dictionary
                let scheme = DictionaryScheme::from_str(parsed_uri.scheme()).map_err(|err| {
                    LinderaErrorKind::Dictionary
                        .with_error(anyhow::anyhow!("Invalid dictionary scheme: {}", err))
                })?;

                match scheme {
                    DictionaryScheme::File => {
                        // Extract path from file:// URL manually
                        let path_str = parsed_uri.path();

                        // Handle Windows paths that might start with /C:/ etc.
                        let path_str =
                            if cfg!(windows) && path_str.len() > 1 && path_str.starts_with('/') {
                                &path_str[1..]
                            } else {
                                path_str
                            };

                        // Decode percent-encoded characters
                        let decoded_path =
                            percent_decode_str(path_str).decode_utf8().map_err(|e| {
                                LinderaErrorKind::Dictionary
                                    .with_error(anyhow::anyhow!("Invalid UTF-8 in path: {}", e))
                            })?;

                        PathBuf::from(decoded_path.as_ref())
                    }
                    #[cfg(any(
                        feature = "embedded-ipadic",
                        feature = "embedded-ipadic-neologd",
                        feature = "embedded-unidic",
                        feature = "embedded-ko-dic",
                        feature = "embedded-cc-cedict",
                    ))]
                    _ => {
                        // Unsupported dictionary scheme
                        return Err(LinderaErrorKind::Dictionary
                            .with_error(anyhow::anyhow!("Unsupported dictionary scheme")));
                    }
                }
            }
            Err(e) => {
                return Err(LinderaErrorKind::Dictionary
                    .with_error(anyhow::anyhow!("Invalid URI format: {}", e)));
            }
        }
    } else {
        // Treat it as a file path directly
        PathBuf::from(uri)
    };

    // extract file extension
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("Invalid user dictionary source file"))
        })?;

    match extension {
        "csv" => load_user_dictionary_from_csv(metadata, &path),
        "bin" => load_user_dictionary_from_bin(&path),
        _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
            "Invalid user dictionary source file extension"
        ))),
    }
}
