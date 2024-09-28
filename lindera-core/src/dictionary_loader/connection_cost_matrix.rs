use std::path::Path;

use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::util::read_file;
use crate::LinderaResult;

pub struct ConnectionCostMatrixLoader {}

impl ConnectionCostMatrixLoader {
    pub fn load(&self, dir: &Path) -> LinderaResult<ConnectionCostMatrix> {
        let path = dir.join("matrix.mtx");
        let data = read_file(path.as_path())?;

        Ok(ConnectionCostMatrix::load(data.as_slice()))
    }
}
