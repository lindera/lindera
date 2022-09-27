use std::path::Path;

use lindera_cc_cedict_builder::cc_cedict_builder::CcCedictBuilder;
use lindera_core::{
    dictionary::Dictionary, dictionary_builder::DictionaryBuilder, file_util::read_file,
    user_dictionary::UserDictionary,
};
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;
use lindera_ko_dic_builder::ko_dic_builder::KoDicBuilder;
use lindera_unidic_builder::unidic_builder::UnidicBuilder;

use crate::{
    error::LinderaErrorKind,
    tokenizer::{DictionaryConfig, UserDictionaryConfig},
    DictionaryKind, LinderaResult,
};

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

/// Conta
pub fn load_dictionary(dictionary_config: DictionaryConfig) -> LinderaResult<Dictionary> {
    let dictionary = match dictionary_config.kind {
        Some(kind) => {
            // The dictionary specified by the feature flag will be loaded.
            match kind {
                #[cfg(feature = "ipadic")]
                DictionaryKind::IPADIC => lindera_ipadic::load_dictionary()
                    .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
                #[cfg(feature = "unidic")]
                DictionaryKind::UniDic => lindera_unidic::load_dictionary()
                    .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
                #[cfg(feature = "ko-dic")]
                DictionaryKind::KoDic => lindera_ko_dic::load_dictionary()
                    .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
                #[cfg(feature = "cc-cedict")]
                DictionaryKind::CcCedict => lindera_cc_cedict::load_dictionary()
                    .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
                #[allow(unreachable_patterns)]
                _ => {
                    return Err(LinderaErrorKind::Args
                        .with_error(anyhow::anyhow!("Invalid dictionary type: {:?}", kind)))
                }
            }
        }
        None => {
            match dictionary_config.path {
                Some(path) => {
                    // load external dictionary from path
                    lindera_dictionary::load_dictionary(path)?
                }
                None => {
                    return Err(LinderaErrorKind::Args
                        .with_error(anyhow::anyhow!("Dictionary must be specified")))
                }
            }
        }
    };

    Ok(dictionary)
}

pub fn load_user_dictionary(
    dictionary_config: UserDictionaryConfig,
) -> LinderaResult<UserDictionary> {
    let user_dictionary = match dictionary_config.path.extension() {
        Some(ext) => match ext.to_str() {
            Some("csv") => match dictionary_config.kind {
                Some(kind) => resolve_builder(kind)?
                    .build_user_dict(&dictionary_config.path)
                    .map_err(|err| LinderaErrorKind::DictionaryBuildError.with_error(err))?,
                None => {
                    return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                        "Dictionary type must be specified if CSV file specified"
                    )))
                }
            },
            Some("bin") => UserDictionary::load(&read_file(&dictionary_config.path)?)?,
            _ => {
                return Err(LinderaErrorKind::Args
                    .with_error(anyhow::anyhow!("Invalid user dictionary source type")))
            }
        },
        None => {
            return Err(LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("Invalid user dictionary source type")))
        }
    };

    Ok(user_dictionary)
}
