use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
#[cfg(feature = "mmap")]
use crate::util::mmap_file;
use crate::util::read_file;

/// Loader for connection cost matrix data from disk files.
pub struct ConnectionCostMatrixLoader {}

impl ConnectionCostMatrixLoader {
    /// Load connection cost matrix from a file in the specified directory.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Path to the directory containing matrix.mtx.
    ///
    /// # Returns
    ///
    /// A `ConnectionCostMatrix` loaded from the file.
    pub fn load(input_dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let data = read_file(input_dir.join("matrix.mtx").as_path())?;

        Ok(ConnectionCostMatrix::load(data))
    }

    /// Load connection cost matrix using memory-mapped file.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Path to the directory containing matrix.mtx.
    ///
    /// # Returns
    ///
    /// A `ConnectionCostMatrix` loaded via memory mapping.
    #[cfg(feature = "mmap")]
    pub fn load_mmap(input_dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let data = mmap_file(input_dir.join("matrix.mtx").as_path())?;

        Ok(ConnectionCostMatrix::load(data))
    }
}
