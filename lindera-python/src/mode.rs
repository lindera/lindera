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
//! A `Mode` holds the penalty of decompose mode, as the Rust
//! `lindera::mode::Mode` does, and is what `TokenizerBuilder.set_mode` and
//! the `Tokenizer` constructor take.
//!
//! # Examples
//!
//! ```python
//! # Normal mode
//! tokenizer = lindera.TokenizerBuilder().set_mode(lindera.Mode.normal()).build()
//!
//! # Decompose mode
//! tokenizer = lindera.TokenizerBuilder().set_mode(lindera.Mode.decompose()).build()
//!
//! # Decompose mode with a custom penalty
//! penalty = lindera.Penalty(kanji_penalty_length_penalty=0)
//! tokenizer = lindera.TokenizerBuilder().set_mode(lindera.Mode.decompose(penalty)).build()
//!
//! # A mode from its name, such as a configuration value
//! mode = lindera.Mode.from_str("decompose")
//! ```

use pyo3::prelude::*;

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

use crate::error::to_py_error;

/// Tokenization mode: normal, or decompose with a `Penalty`.
///
/// The Python counterpart of `lindera::mode::Mode`. It has no constructor;
/// `Mode.normal()`, `Mode.decompose()` and `Mode.from_str()` create one. A
/// `Mode` cannot be changed, and two modes are equal when their names and
/// penalties are. Like the Rust `Mode`, it is not hashable.
#[pyclass(name = "Mode", frozen, eq, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct PyMode {
    /// The mode, with its penalty for decompose mode.
    inner: LinderaMode,
}

impl PyMode {
    /// Returns the wrapped `lindera::mode::Mode`.
    pub fn as_lindera(&self) -> &LinderaMode {
        &self.inner
    }
}

#[pymethods]
impl PyMode {
    /// Creates normal mode.
    ///
    /// # Returns
    ///
    /// Normal mode.
    #[staticmethod]
    pub fn normal() -> Self {
        Self {
            inner: LinderaMode::Normal,
        }
    }

    /// Creates decompose mode.
    ///
    /// # Arguments
    ///
    /// * `penalty` - The `Penalty`. `None` (the default) uses the default
    ///   penalty.
    ///
    /// # Returns
    ///
    /// Decompose mode with the penalty.
    #[staticmethod]
    #[pyo3(signature = (penalty=None))]
    pub fn decompose(penalty: Option<PyPenalty>) -> Self {
        Self {
            inner: LinderaMode::Decompose(penalty.map(Into::into).unwrap_or_default()),
        }
    }

    /// Creates a mode from its name, as the Rust `Mode` parses one.
    ///
    /// # Arguments
    ///
    /// * `name` - `"normal"` or `"decompose"`, case sensitive.
    ///
    /// # Returns
    ///
    /// The mode, with the default penalty for `"decompose"`, or `ValueError`
    /// for any other name.
    #[staticmethod]
    #[pyo3(name = "from_str", signature = (name))]
    pub fn parse(name: &str) -> PyResult<Self> {
        let inner = lindera_binding::parse_mode(name).map_err(to_py_error)?;
        Ok(Self { inner })
    }

    /// The name of the mode: `"normal"` or `"decompose"`.
    #[getter]
    pub fn name(&self) -> &str {
        self.inner.as_str()
    }

    /// The penalty of decompose mode, as a copy, or `None` for normal mode.
    #[getter]
    pub fn penalty(&self) -> Option<PyPenalty> {
        match &self.inner {
            LinderaMode::Normal => None,
            LinderaMode::Decompose(penalty) => Some(penalty.clone().into()),
        }
    }

    /// Returns whether this is normal mode.
    pub fn is_normal(&self) -> bool {
        matches!(self.inner, LinderaMode::Normal)
    }

    /// Returns whether this is decompose mode.
    pub fn is_decompose(&self) -> bool {
        matches!(self.inner, LinderaMode::Decompose(_))
    }

    fn __str__(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            LinderaMode::Normal => "Mode.normal()".to_string(),
            LinderaMode::Decompose(penalty) => {
                format!(
                    "Mode.decompose({})",
                    PyPenalty::from(penalty.clone()).__repr__()
                )
            }
        }
    }
}

impl From<LinderaMode> for PyMode {
    fn from(inner: LinderaMode) -> Self {
        Self { inner }
    }
}

/// Penalty configuration for decompose mode.
///
/// Controls how aggressively compound words are decomposed based on
/// character type and length thresholds. Pass it to `Mode.decompose`.
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

    /// Sets the kanji length threshold.
    ///
    /// The argument is named after the property because pyo3 names it in
    /// the note of the `OverflowError` for a value outside the range of `usize`
    /// (`while processing 'kanji_penalty_length_threshold'`).
    ///
    /// # Arguments
    ///
    /// * `kanji_penalty_length_threshold` - The new value.
    #[setter]
    pub fn set_kanji_penalty_length_threshold(&mut self, kanji_penalty_length_threshold: usize) {
        self.kanji_penalty_length_threshold = kanji_penalty_length_threshold;
    }

    #[getter]
    pub fn get_kanji_penalty_length_penalty(&self) -> i32 {
        self.kanji_penalty_length_penalty
    }

    /// Sets the kanji penalty value.
    ///
    /// The argument is named after the property because pyo3 names it in
    /// the note of the `OverflowError` for a value outside the range of `i32`
    /// (`while processing 'kanji_penalty_length_penalty'`).
    ///
    /// # Arguments
    ///
    /// * `kanji_penalty_length_penalty` - The new value.
    #[setter]
    pub fn set_kanji_penalty_length_penalty(&mut self, kanji_penalty_length_penalty: i32) {
        self.kanji_penalty_length_penalty = kanji_penalty_length_penalty;
    }

    #[getter]
    pub fn get_other_penalty_length_threshold(&self) -> usize {
        self.other_penalty_length_threshold
    }

    /// Sets the other character length threshold.
    ///
    /// The argument is named after the property because pyo3 names it in
    /// the note of the `OverflowError` for a value outside the range of `usize`
    /// (`while processing 'other_penalty_length_threshold'`).
    ///
    /// # Arguments
    ///
    /// * `other_penalty_length_threshold` - The new value.
    #[setter]
    pub fn set_other_penalty_length_threshold(&mut self, other_penalty_length_threshold: usize) {
        self.other_penalty_length_threshold = other_penalty_length_threshold;
    }

    #[getter]
    pub fn get_other_penalty_length_penalty(&self) -> i32 {
        self.other_penalty_length_penalty
    }

    /// Sets the other character penalty value.
    ///
    /// The argument is named after the property because pyo3 names it in
    /// the note of the `OverflowError` for a value outside the range of `i32`
    /// (`while processing 'other_penalty_length_penalty'`).
    ///
    /// # Arguments
    ///
    /// * `other_penalty_length_penalty` - The new value.
    #[setter]
    pub fn set_other_penalty_length_penalty(&mut self, other_penalty_length_penalty: i32) {
        self.other_penalty_length_penalty = other_penalty_length_penalty;
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
    fn test_pymode_factories() {
        assert_eq!(PyMode::normal().as_lindera(), &LinderaMode::Normal);
        assert_eq!(
            PyMode::decompose(None).as_lindera(),
            &LinderaMode::Decompose(LinderaPenalty::default())
        );
        let penalty = PyPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 0,
            other_penalty_length_threshold: 4,
            other_penalty_length_penalty: 3000,
        };
        let mode = PyMode::decompose(Some(penalty));
        assert_eq!(
            mode.as_lindera(),
            &LinderaMode::Decompose(LinderaPenalty::from(penalty))
        );
        let Some(back) = mode.penalty() else {
            panic!("decompose mode has no penalty");
        };
        assert_eq!(back.kanji_penalty_length_threshold, 3);
        assert_eq!(back.other_penalty_length_penalty, 3000);
        assert!(PyMode::normal().penalty().is_none());
    }

    #[test]
    fn test_pymode_names_and_kinds() {
        let normal = PyMode::normal();
        let decompose = PyMode::decompose(None);
        assert_eq!(normal.name(), "normal");
        assert_eq!(decompose.name(), "decompose");
        assert!(normal.is_normal() && !normal.is_decompose());
        assert!(decompose.is_decompose() && !decompose.is_normal());
        assert_eq!(normal.__repr__(), "Mode.normal()");
        assert!(decompose.__repr__().starts_with("Mode.decompose(Penalty("));
    }

    /// Two modes are equal when their names and penalties are, as the Rust
    /// `Mode` compares.
    #[test]
    fn test_pymode_equality_compares_the_penalty() {
        let zero = PyPenalty {
            kanji_penalty_length_threshold: 2,
            kanji_penalty_length_penalty: 0,
            other_penalty_length_threshold: 7,
            other_penalty_length_penalty: 0,
        };
        assert_eq!(PyMode::decompose(None), PyMode::decompose(None));
        assert_ne!(PyMode::decompose(None), PyMode::decompose(Some(zero)));
        assert_ne!(PyMode::normal(), PyMode::decompose(None));
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
