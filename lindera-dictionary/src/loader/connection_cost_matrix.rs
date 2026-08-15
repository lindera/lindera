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
    /// This is the zero-copy path: an mmap base is page-aligned and
    /// `matrix.mtx` already stores its costs as little-endian `i16` in the
    /// in-memory layout, so [`ConnectionCostMatrix::load`] views the payload
    /// in place instead of decoding it into an owned `Vec<i16>`. Loading is
    /// O(1) in the matrix size and costs no anonymous memory; the pages are
    /// faulted in lazily as tokenization touches them. UniDic's matrix alone
    /// is 71.5 MB, so this is the bulk of that dictionary's load cost.
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
