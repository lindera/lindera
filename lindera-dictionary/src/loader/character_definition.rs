use std::path::Path;

use crate::LinderaResult;
#[cfg(feature = "compress")]
use crate::decompress::{CompressedData, decompress};
use crate::dictionary::character_definition::CharacterDefinition;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
use crate::util::read_file;

pub struct CharacterDefinitionLoader {}

impl CharacterDefinitionLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<CharacterDefinition> {
        let raw_data = read_file(input_dir.join("char_def.bin").as_path())?;

        let mut aligned_data = rkyv::util::AlignedVec::<16>::new();
        aligned_data.extend_from_slice(&raw_data);

        #[cfg(feature = "compress")]
        {
            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned_data).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize char_def.bin data")
                    },
                )?;

            let decompressed_data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress character definition data")
            })?;

            let mut aligned_decompressed = rkyv::util::AlignedVec::<16>::new();
            aligned_decompressed.extend_from_slice(&decompressed_data);

            CharacterDefinition::load(&aligned_decompressed)
        }

        #[cfg(not(feature = "compress"))]
        {
            CharacterDefinition::load(&aligned_data)
        }
    }
}
