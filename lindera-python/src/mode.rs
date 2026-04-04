//! Tokenization modes and penalty configurations.
//!
//! This module defines the different tokenization modes available and their
//! penalty configurations for controlling segmentation behavior.
//!
//! # Modes
//!
//! - **Normal**: Standard tokenization based on dictionary cost
//! - **Decompose**: Decomposes compound words with penalty-based control
//!
//! # Examples
//!
//! ```python
//! # Normal mode
//! tokenizer = lindera.TokenizerBuilder().set_mode("normal").build()
//!
//! # Decompose mode
//! tokenizer = lindera.TokenizerBuilder().set_mode("decompose").build()
//!
//! # Custom penalty configuration
//! penalty = lindera.Penalty(
//!     kanji_penalty_length_threshold=2,
//!     kanji_penalty_length_penalty=3000
//! )
//! ```

use pyo3::prelude::*;

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

/// Tokenization mode.
///
/// Determines how text is segmented into tokens.
#[pyclass(name = "Mode", from_py_object)]
#[derive(Debug, Clone, Copy)]
pub enum PyMode {
    /// Standard tokenization based on dictionary cost
    Normal,
    /// Decompose compound words using penalty-based segmentation
    Decompose,
}

#[pymethods]
impl PyMode {
    #[new]
    #[pyo3(signature = (mode_str=None))]
    pub fn new(mode_str: Option<&str>) -> PyResult<Self> {
        match mode_str {
            Some("decompose") | Some("Decompose") => Ok(PyMode::Decompose),
            Some("normal") | Some("Normal") | None => Ok(PyMode::Normal),
            Some(s) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid mode: {s}. Must be 'normal' or 'decompose'"
            ))),
        }
    }

    fn __str__(&self) -> &str {
        match self {
            PyMode::Normal => "normal",
            PyMode::Decompose => "decompose",
        }
    }

    fn __repr__(&self) -> String {
        format!("Mode.{self:?}")
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.__str__()
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, PyMode::Normal)
    }

    pub fn is_decompose(&self) -> bool {
        matches!(self, PyMode::Decompose)
    }
}

impl From<PyMode> for LinderaMode {
    fn from(mode: PyMode) -> Self {
        match mode {
            PyMode::Normal => LinderaMode::Normal,
            PyMode::Decompose => LinderaMode::Decompose(LinderaPenalty::default()),
        }
    }
}

impl From<LinderaMode> for PyMode {
    fn from(mode: LinderaMode) -> Self {
        match mode {
            LinderaMode::Normal => PyMode::Normal,
            LinderaMode::Decompose(_) => PyMode::Decompose,
        }
    }
}

/// Penalty configuration for decompose mode.
///
/// Controls how aggressively compound words are decomposed based on
/// character type and length thresholds.
///
/// # Examples
///
/// ```python
/// penalty = lindera.Penalty(
///     kanji_penalty_length_threshold=2,
///     kanji_penalty_length_penalty=3000,
///     other_penalty_length_threshold=7,
///     other_penalty_length_penalty=1700
/// )
/// ```
#[pyclass(name = "Penalty", from_py_object)]
#[derive(Debug, Clone, Copy)]
pub struct PyPenalty {
    kanji_penalty_length_threshold: usize,
    kanji_penalty_length_penalty: i32,
    other_penalty_length_threshold: usize,
    other_penalty_length_penalty: i32,
}

#[pymethods]
impl PyPenalty {
    #[new]
    #[pyo3(signature = (kanji_penalty_length_threshold=None, kanji_penalty_length_penalty=None, other_penalty_length_threshold=None, other_penalty_length_penalty=None))]
    pub fn new(
        kanji_penalty_length_threshold: Option<usize>,
        kanji_penalty_length_penalty: Option<i32>,
        other_penalty_length_threshold: Option<usize>,
        other_penalty_length_penalty: Option<i32>,
    ) -> Self {
        PyPenalty {
            kanji_penalty_length_threshold: kanji_penalty_length_threshold.unwrap_or(2),
            kanji_penalty_length_penalty: kanji_penalty_length_penalty.unwrap_or(3000),
            other_penalty_length_threshold: other_penalty_length_threshold.unwrap_or(7),
            other_penalty_length_penalty: other_penalty_length_penalty.unwrap_or(1700),
        }
    }

    #[getter]
    pub fn get_kanji_penalty_length_threshold(&self) -> usize {
        self.kanji_penalty_length_threshold
    }

    #[setter]
    pub fn set_kanji_penalty_length_threshold(&mut self, value: usize) {
        self.kanji_penalty_length_threshold = value;
    }

    #[getter]
    pub fn get_kanji_penalty_length_penalty(&self) -> i32 {
        self.kanji_penalty_length_penalty
    }

    #[setter]
    pub fn set_kanji_penalty_length_penalty(&mut self, value: i32) {
        self.kanji_penalty_length_penalty = value;
    }

    #[getter]
    pub fn get_other_penalty_length_threshold(&self) -> usize {
        self.other_penalty_length_threshold
    }

    #[setter]
    pub fn set_other_penalty_length_threshold(&mut self, value: usize) {
        self.other_penalty_length_threshold = value;
    }

    #[getter]
    pub fn get_other_penalty_length_penalty(&self) -> i32 {
        self.other_penalty_length_penalty
    }

    #[setter]
    pub fn set_other_penalty_length_penalty(&mut self, value: i32) {
        self.other_penalty_length_penalty = value;
    }

    fn __str__(&self) -> String {
        format!(
            "Penalty(kanji_threshold={}, kanji_penalty={}, other_threshold={}, other_penalty={})",
            self.kanji_penalty_length_threshold,
            self.kanji_penalty_length_penalty,
            self.other_penalty_length_threshold,
            self.other_penalty_length_penalty
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl From<PyPenalty> for LinderaPenalty {
    fn from(penalty: PyPenalty) -> Self {
        LinderaPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

impl From<LinderaPenalty> for PyPenalty {
    fn from(penalty: LinderaPenalty) -> Self {
        PyPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "mode")?;
    m.add_class::<PyMode>()?;
    m.add_class::<PyPenalty>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

    #[test]
    fn test_pymode_normal_to_lindera_mode() {
        let py_mode = PyMode::Normal;
        let lindera_mode: LinderaMode = py_mode.into();
        assert!(matches!(lindera_mode, LinderaMode::Normal));
    }

    #[test]
    fn test_pymode_decompose_to_lindera_mode() {
        let py_mode = PyMode::Decompose;
        let lindera_mode: LinderaMode = py_mode.into();
        assert!(matches!(lindera_mode, LinderaMode::Decompose(_)));
    }

    #[test]
    fn test_lindera_mode_normal_to_pymode() {
        let lindera_mode = LinderaMode::Normal;
        let py_mode: PyMode = lindera_mode.into();
        assert!(matches!(py_mode, PyMode::Normal));
    }

    #[test]
    fn test_lindera_mode_decompose_to_pymode() {
        let lindera_mode = LinderaMode::Decompose(LinderaPenalty::default());
        let py_mode: PyMode = lindera_mode.into();
        assert!(matches!(py_mode, PyMode::Decompose));
    }

    #[test]
    fn test_pypenalty_to_lindera_penalty() {
        let py_penalty = PyPenalty {
            kanji_penalty_length_threshold: 5,
            kanji_penalty_length_penalty: 4000,
            other_penalty_length_threshold: 10,
            other_penalty_length_penalty: 2000,
        };
        let lindera_penalty: LinderaPenalty = py_penalty.into();
        assert_eq!(lindera_penalty.kanji_penalty_length_threshold, 5);
        assert_eq!(lindera_penalty.kanji_penalty_length_penalty, 4000);
        assert_eq!(lindera_penalty.other_penalty_length_threshold, 10);
        assert_eq!(lindera_penalty.other_penalty_length_penalty, 2000);
    }

    #[test]
    fn test_lindera_penalty_to_pypenalty() {
        let lindera_penalty = LinderaPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 5000,
            other_penalty_length_threshold: 8,
            other_penalty_length_penalty: 1500,
        };
        let py_penalty: PyPenalty = lindera_penalty.into();
        assert_eq!(py_penalty.kanji_penalty_length_threshold, 3);
        assert_eq!(py_penalty.kanji_penalty_length_penalty, 5000);
        assert_eq!(py_penalty.other_penalty_length_threshold, 8);
        assert_eq!(py_penalty.other_penalty_length_penalty, 1500);
    }

    #[test]
    fn test_pypenalty_to_lindera_penalty_default_values() {
        let py_penalty = PyPenalty {
            kanji_penalty_length_threshold: 2,
            kanji_penalty_length_penalty: 3000,
            other_penalty_length_threshold: 7,
            other_penalty_length_penalty: 1700,
        };
        let lindera_penalty: LinderaPenalty = py_penalty.into();
        let default_penalty = LinderaPenalty::default();
        assert_eq!(
            lindera_penalty.kanji_penalty_length_threshold,
            default_penalty.kanji_penalty_length_threshold
        );
        assert_eq!(
            lindera_penalty.kanji_penalty_length_penalty,
            default_penalty.kanji_penalty_length_penalty
        );
        assert_eq!(
            lindera_penalty.other_penalty_length_threshold,
            default_penalty.other_penalty_length_threshold
        );
        assert_eq!(
            lindera_penalty.other_penalty_length_penalty,
            default_penalty.other_penalty_length_penalty
        );
    }

    #[test]
    fn test_pypenalty_roundtrip() {
        let original = PyPenalty {
            kanji_penalty_length_threshold: 4,
            kanji_penalty_length_penalty: 2500,
            other_penalty_length_threshold: 6,
            other_penalty_length_penalty: 1800,
        };
        let lindera: LinderaPenalty = original.into();
        let roundtripped: PyPenalty = lindera.into();
        assert_eq!(roundtripped.kanji_penalty_length_threshold, 4);
        assert_eq!(roundtripped.kanji_penalty_length_penalty, 2500);
        assert_eq!(roundtripped.other_penalty_length_threshold, 6);
        assert_eq!(roundtripped.other_penalty_length_penalty, 1800);
    }
}
