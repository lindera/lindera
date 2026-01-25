//! Character filters for preprocessing text.

use pyo3::prelude::*;

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "character_filter")?;
    // Add character filter related classes here in the future
    parent_module.add_submodule(&m)?;
    Ok(())
}
