use std::path::Path;

use crate::LinderaResult;
#[cfg(feature = "compress")]
use crate::decompress::{CompressedData, decompress};
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
        let mut vals_costs_data = read_file(input_dir.join("dict.vals.cost").as_path())?;
        let mut vals_left_ids_data = read_file(input_dir.join("dict.vals.left").as_path())?;
        let mut vals_right_ids_data = read_file(input_dir.join("dict.vals.right").as_path())?;
        let mut vals_word_ids_data = read_file(input_dir.join("dict.vals.idx").as_path())?;
        let mut words_idx_data = read_file(input_dir.join("dict.wordsidx").as_path())?;
        let mut words_data = read_file(input_dir.join("dict.words").as_path())?;

        #[cfg(feature = "compress")]
        {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&da_data);
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize dict.da data")
                    },
                )?;
            da_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.da DoubleArray data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&vals_costs_data);
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize dict.vals.cost data")
                    },
                )?;
            vals_costs_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.vals.cost word values data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&vals_left_ids_data);
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize dict.vals.left data")
                    },
                )?;
            vals_left_ids_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.vals.left word values data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&vals_right_ids_data);
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize dict.vals.right data")
                    },
                )?;
            vals_right_ids_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.vals.right word values data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&vals_word_ids_data);
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize dict.vals.idx data")
                    },
                )?;
            vals_word_ids_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.vals.idx word values data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&words_idx_data);
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize dict.wordsidx data")
                    },
                )?;
            words_idx_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.wordsidx word index data")
            })?;
        }
        #[cfg(feature = "compress")]
        {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&words_data);
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize dict.words data")
                    },
                )?;
            words_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress dict.words word details data")
            })?;
        }

        Ok(PrefixDictionary::load(
            da_data,
            vals_costs_data,
            vals_left_ids_data,
            vals_right_ids_data,
            vals_word_ids_data,
            words_idx_data,
            words_data,
            true,
        ))
    }

    #[cfg(feature = "mmap")]
    pub fn load_mmap(input_dir: &Path) -> LinderaResult<PrefixDictionary> {
        let da_data = mmap_file(input_dir.join("dict.da").as_path())?;
        let vals_costs_data = mmap_file(input_dir.join("dict.vals.cost").as_path())?;
        let vals_left_ids_data = mmap_file(input_dir.join("dict.vals.left").as_path())?;
        let vals_right_ids_data = mmap_file(input_dir.join("dict.vals.right").as_path())?;
        let vals_word_ids_data = mmap_file(input_dir.join("dict.vals.idx").as_path())?;
        let words_idx_data = mmap_file(input_dir.join("dict.wordsidx").as_path())?;
        let words_data = mmap_file(input_dir.join("dict.words").as_path())?;

        Ok(PrefixDictionary::load(
            da_data,
            vals_costs_data,
            vals_left_ids_data,
            vals_right_ids_data,
            vals_word_ids_data,
            words_idx_data,
            words_data,
            true,
        ))
    }
}
