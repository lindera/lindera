use std::path::Path;

use crate::LinderaResult;
#[cfg(feature = "compress")]
use crate::decompress::decompress;
use crate::dictionary::prefix_dictionary::PrefixDictionary;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
#[cfg(feature = "mmap")]
use crate::util::mmap_file;
use crate::util::read_file;

pub struct PrefixDictionaryLoader {}

impl PrefixDictionaryLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<PrefixDictionary> {
        let mut da_data = read_file(input_dir.join("dict.da").as_path())?;
        let mut vals_data = read_file(input_dir.join("dict.vals").as_path())?;
        let mut words_idx_data = read_file(input_dir.join("dict.wordsidx").as_path())?;
        let mut words_data = read_file(input_dir.join("dict.words").as_path())?;

        #[cfg(feature = "compress")]
        {
            let (compressed_data, _) =
                bincode::serde::decode_from_slice(da_data.as_slice(), bincode::config::legacy())
                    .map_err(|err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err))
                            .add_context("Failed to deserialize dict.da data")
                    })?;
            da_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.da DoubleArray data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let (compressed_data, _) =
                bincode::serde::decode_from_slice(vals_data.as_slice(), bincode::config::legacy())
                    .map_err(|err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err))
                            .add_context("Failed to deserialize dict.vals data")
                    })?;
            vals_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.vals word values data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let (compressed_data, _) = bincode::serde::decode_from_slice(
                words_idx_data.as_slice(),
                bincode::config::legacy(),
            )
            .map_err(|err| {
                LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!(err))
                    .add_context("Failed to deserialize dict.wordsidx data")
            })?;
            words_idx_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.wordsidx word index data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let (compressed_data, _) =
                bincode::serde::decode_from_slice(words_data.as_slice(), bincode::config::legacy())
                    .map_err(|err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err))
                            .add_context("Failed to deserialize dict.words data")
                    })?;
            words_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.words word details data")
            })?;
        }

        Ok(PrefixDictionary::load(
            da_data,
            vals_data,
            words_idx_data,
            words_data,
            true,
        ))
    }

    #[cfg(feature = "mmap")]
    pub fn load_mmap(input_dir: &Path) -> LinderaResult<PrefixDictionary> {
        let da_data = mmap_file(input_dir.join("dict.da").as_path())?;
        let vals_data = mmap_file(input_dir.join("dict.vals").as_path())?;
        let words_idx_data = mmap_file(input_dir.join("dict.wordsidx").as_path())?;
        let words_data = mmap_file(input_dir.join("dict.words").as_path())?;

        Ok(PrefixDictionary::load(
            da_data,
            vals_data,
            words_idx_data,
            words_data,
            true,
        ))
    }
}
