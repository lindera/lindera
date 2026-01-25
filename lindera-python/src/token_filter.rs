//! Token filters for post-processing tokens.

use pyo3::prelude::*;

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "token_filter")?;
    // Add token filter related classes here in the future
    parent_module.add_submodule(&m)?;
    Ok(())
}
