use std::path::Path;

#[cfg(feature = "compress")]
use crate::decompress::decompress;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
#[cfg(feature = "mmap")]
use crate::util::mmap_file;
use crate::util::read_file;
use crate::LinderaResult;

pub struct ConnectionCostMatrixLoader {}

impl ConnectionCostMatrixLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let mut data = read_file(input_dir.join("matrix.mtx").as_path())?;

        #[cfg(feature = "compress")]
        {
            let (compressed_data, _) =
                bincode::serde::decode_from_slice(data.as_slice(), bincode::config::legacy())
                    .map_err(|err| {
                        LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err))
                    })?;
            data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }

        Ok(ConnectionCostMatrix::load(data))
    }

    #[cfg(feature = "mmap")]
    pub fn load_mmap(input_dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let data = mmap_file(input_dir.join("matrix.mtx").as_path())?;

        Ok(ConnectionCostMatrix::load(data))
    }
}
