use std::path::Path;

use crate::LinderaResult;
#[cfg(feature = "compress")]
use crate::decompress::{CompressedData, decompress};
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
#[cfg(feature = "mmap")]
use crate::util::mmap_file;
use crate::util::read_file;

pub struct ConnectionCostMatrixLoader {}

impl ConnectionCostMatrixLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let mut data = read_file(input_dir.join("matrix.mtx").as_path())?;

        #[cfg(feature = "compress")]
        {
            let mut aligned_data = rkyv::util::AlignedVec::<16>::new();
            aligned_data.extend_from_slice(&data);

            let compressed_data: CompressedData =
                rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned_data).map_err(
                    |err| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!(err.to_string()))
                            .add_context("Failed to deserialize matrix.mtx data")
                    },
                )?;
            data = decompress(compressed_data).map_err(|err| {
                LinderaErrorKind::Compression
                    .with_error(err)
                    .add_context("Failed to decompress connection cost matrix data")
            })?;
        }

        Ok(ConnectionCostMatrix::load(data))
    }

    #[cfg(feature = "mmap")]
    pub fn load_mmap(input_dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let data = mmap_file(input_dir.join("matrix.mtx").as_path())?;

        Ok(ConnectionCostMatrix::load(data))
    }
}
