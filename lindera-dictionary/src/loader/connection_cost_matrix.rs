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

        ConnectionCostMatrix::load(data)
    }

    /// Load connection cost matrix using memory-mapped file.
    ///
    /// Note: mmap only avoids the initial file-read syscall/allocation.
    /// [`ConnectionCostMatrix::load`] always eagerly decodes the whole buffer
    /// into an owned `Vec<i16>` regardless of source (by design, to make the
    /// hot-path `cost()` lookup a plain array index), so this does not make
    /// the matrix lazily memory-resident at runtime.
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

        ConnectionCostMatrix::load(data)
    }
}
