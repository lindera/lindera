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
            "<Token surface='{}', start={}, end={}, position={}, word_id={}>",
            self.surface, self.byte_start, self.byte_end, self.position, self.word_id
        )
    }
}

impl PyToken {
    pub fn from_token(mut token: Token) -> Self {
        let details = token.details().iter().map(|s| s.to_string()).collect();
        // Since lindera::token::Token.details() returns Vec<&str>, we convert to Vec<String>.
        // Wait, Token.details() actually calls ensure_details() which loads from dictionary.

        Self {
            surface: token.surface.to_string(),
            byte_start: token.byte_start,
            byte_end: token.byte_end,
            position: token.position,
            word_id: token.word_id.id,
            details: Some(details),
        }
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "token")?;
    m.add_class::<PyToken>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}
