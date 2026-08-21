use pyo3::prelude::*;
use pyo3::types::PyDict;

use lindera::token::Token;

/// Token object wrapping the Rust Token data.
///
/// This class provides robust access to token field and details.
#[pyclass(name = "Token")]
pub struct PyToken {
    /// Surface form of the token.
    #[pyo3(get)]
    pub surface: String,

    /// Start byte position in the original text.
    #[pyo3(get)]
    pub byte_start: usize,

    /// End byte position in the original text.
    #[pyo3(get)]
    pub byte_end: usize,

    /// Position index of the token.
    #[pyo3(get)]
    pub position: usize,

    /// Word ID in the dictionary.
    #[pyo3(get)]
    pub word_id: u32,

    /// Whether this token is an unknown word (not found in the dictionary).
    #[pyo3(get)]
    pub is_unknown: bool,

    /// Morphological details of the token.
    #[pyo3(get)]
    pub details: Vec<String>,
}

#[pymethods]
impl PyToken {
    /// Returns the detail at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the detail to retrieve.
    ///
    /// # Returns
    ///
    /// The detail string if found, otherwise None.
    #[pyo3(signature = (index))]
    fn get_detail(&self, index: usize) -> Option<String> {
        self.details.get(index).cloned()
    }

    /// Returns the token as a plain dictionary.
    ///
    /// Field values keep their natural Python types (`int` for the
    /// positions and word id, `bool` for `is_unknown`, `list[str]` for
    /// `details`), so the result serializes directly with `json.dumps`.
    ///
    /// # Returns
    ///
    /// A dict with the keys `surface`, `byte_start`, `byte_end`,
    /// `position`, `word_id`, `is_unknown`, and `details`.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("surface", &self.surface)?;
        dict.set_item("byte_start", self.byte_start)?;
        dict.set_item("byte_end", self.byte_end)?;
        dict.set_item("position", self.position)?;
        dict.set_item("word_id", self.word_id)?;
        dict.set_item("is_unknown", self.is_unknown)?;
        dict.set_item("details", &self.details)?;
        Ok(dict)
    }

    /// Returns a string representation of the token.
    fn __repr__(&self) -> String {
        format!(
            "<Token surface='{}', start={}, end={}, position={}, word_id={}, is_unknown={}>",
            self.surface,
            self.byte_start,
            self.byte_end,
            self.position,
            self.word_id,
            self.is_unknown
        )
    }
}

impl PyToken {
    /// Builds a `PyToken` from a binding-core [`TokenView`].
    pub fn from_view(view: lindera_binding_core::TokenView) -> Self {
        Self {
            surface: view.surface,
            byte_start: view.byte_start,
            byte_end: view.byte_end,
            position: view.position,
            word_id: view.word_id,
            is_unknown: view.is_unknown,
            details: view.details,
        }
    }

    /// Builds a `PyToken` from a `lindera` token via [`TokenView`].
    pub fn from_token(token: Token) -> Self {
        Self::from_view(lindera_binding_core::TokenView::from_token(token))
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "token")?;
    m.add_class::<PyToken>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}
