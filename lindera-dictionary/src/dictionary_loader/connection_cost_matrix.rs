use std::path::Path;

#[cfg(feature = "compress")]
use crate::decompress::decompress;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
#[cfg(feature = "memmap")]
use crate::util::memmap_file;
use crate::util::read_file;
use crate::LinderaResult;

pub struct ConnectionCostMatrixLoader {}

impl ConnectionCostMatrixLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let mut data = read_file(input_dir.join("matrix.mtx").as_path())?;

        #[cfg(feature = "compress")]
        {
            let compressed_data = bincode::deserialize_from(data.as_slice())
                .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
            data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }

        Ok(ConnectionCostMatrix::load(data))
    }

    #[cfg(feature = "memmap")]
    pub fn load_memmap(input_dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let data = memmap_file(input_dir.join("matrix.mtx").as_path())?;

        Ok(ConnectionCostMatrix::load(data))
    }
}
