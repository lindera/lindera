//! Segmenter implementation for morphological analysis.

use pyo3::prelude::*;

/// Core segmenter for morphological analysis.
#[pyclass(name = "Segmenter")]
#[derive(Clone)]
pub struct PySegmenter {
    pub inner: lindera::segmenter::Segmenter,
}

#[pymethods]
impl PySegmenter {
    fn __repr__(&self) -> String {
        "Segmenter()".to_string()
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "segmenter")?;
    m.add_class::<PySegmenter>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}
