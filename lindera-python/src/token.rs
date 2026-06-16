use pyo3::prelude::*;

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
    pub details: Option<Vec<String>>,
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
        self.details.as_ref().and_then(|d| d.get(index).cloned())
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
            details: Some(view.details),
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
