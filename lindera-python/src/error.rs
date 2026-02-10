//! Error types for Lindera operations.
//!
//! This module provides error types used throughout the Lindera Python bindings.

use std::fmt;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;

/// Error type for Lindera operations.
///
/// Represents errors that can occur during tokenization, dictionary operations,
/// or other Lindera functionality.
#[pyclass(name = "LinderaError", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyLinderaError {
    message: String,
}

#[pymethods]
impl PyLinderaError {
    #[new]
    pub fn new(message: String) -> Self {
        PyLinderaError { message }
    }

    #[getter]
    pub fn message(&self) -> &str {
        &self.message
    }

    fn __str__(&self) -> String {
        self.message.clone()
    }

    fn __repr__(&self) -> String {
        format!("LinderaError('{}')", self.message)
    }
}

impl fmt::Display for PyLinderaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PyLinderaError {}

impl From<PyLinderaError> for PyErr {
    fn from(err: PyLinderaError) -> PyErr {
        PyException::new_err(err.message)
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "error")?;
    m.add_class::<PyLinderaError>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}
