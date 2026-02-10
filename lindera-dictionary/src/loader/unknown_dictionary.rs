use std::path::Path;

use crate::LinderaResult;
#[cfg(feature = "compress")]
use crate::decompress::{CompressedData, decompress};
use crate::dictionary::unknown_dictionary::UnknownDictionary;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
use crate::util::read_file;

pub struct UnknownDictionaryLoader {}

impl UnknownDictionaryLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<UnknownDictionary> {
        let raw_data = read_file(input_dir.join("unk.bin").as_path())?;

        let mut aligned_data = rkyv::util::AlignedVec::<16>::new();
        aligned_data.extend_from_slice(&raw_data);

        #[cfg(feature = "compress")]
        {
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned_data).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize unk.bin data")
                    },
                )?;

            let decompressed_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress unknown dictionary data")
            })?;

            let mut aligned_decompressed = rkyv::util::AlignedVec::<16>::new();
            aligned_decompressed.extend_from_slice(&decompressed_data);

            UnknownDictionary::load(&aligned_decompressed)
        }

        #[cfg(not(feature = "compress"))]
        {
            UnknownDictionary::load(&aligned_data)
        }
    }
}
