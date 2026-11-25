use pyo3::prelude::*;
use pyo3::types::PyDict;

use lindera::token::Token;

/// The bunsetsu of a token and what that bunsetsu depends on: what the
/// token filter `japanese_dependency` writes on a token.
///
/// The bunsetsu of a sentence are numbered from 0, and each of them but the
/// last depends on a later one, its head.
#[pyclass(name = "TokenDependency", frozen, get_all)]
#[derive(Clone)]
pub struct PyTokenDependency {
    /// The number of the sentence in the text, from 0.
    pub sentence: u32,

    /// The number of the bunsetsu of the token in its sentence, from 0.
    pub bunsetsu: u32,

    /// The number of the bunsetsu that the bunsetsu of the token depends
    /// on, or None for the last bunsetsu of a sentence.
    pub head: Option<u32>,

    /// What the token is in its bunsetsu: `head`, `function`,
    /// `head_function` or `other`.
    pub role: &'static str,

    /// The score of the decision that attached the bunsetsu to its head; 0
    /// where there was nothing to decide.
    pub score: f64,
}

#[pymethods]
impl PyTokenDependency {
    /// Returns the dependency as a plain dictionary.
    ///
    /// # Returns
    ///
    /// A dict with the keys `sentence`, `bunsetsu`, `head` (None for the
    /// last bunsetsu of a sentence), `role` and `score`.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("sentence", self.sentence)?;
        dict.set_item("bunsetsu", self.bunsetsu)?;
        dict.set_item("head", self.head)?;
        dict.set_item("role", self.role)?;
        dict.set_item("score", self.score)?;
        Ok(dict)
    }

    /// Returns a string representation of the dependency.
    fn __repr__(&self) -> String {
        let head = match self.head {
            Some(head) => head.to_string(),
            None => "None".to_string(),
        };
        format!(
            "<TokenDependency sentence={}, bunsetsu={}, head={}, role='{}', score={}>",
            self.sentence, self.bunsetsu, head, self.role, self.score
        )
    }
}

impl PyTokenDependency {
    /// Builds a `PyTokenDependency` from a lindera-binding
    /// [`DependencyView`](lindera_binding::DependencyView).
    fn from_view(view: lindera_binding::DependencyView) -> Self {
        Self {
            sentence: view.sentence,
            bunsetsu: view.bunsetsu,
            head: view.head,
            role: view.role(),
            score: view.score,
        }
    }
}

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

    /// The bunsetsu of the token and its head, or None unless the token
    /// filter `japanese_dependency` ran.
    #[pyo3(get)]
    pub dependency: Option<PyTokenDependency>,
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
    /// `position`, `word_id`, `is_unknown`, `details`, and `dependency`:
    /// None, or the dict of
    /// `TokenDependency.to_dict`.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("surface", &self.surface)?;
        dict.set_item("byte_start", self.byte_start)?;
        dict.set_item("byte_end", self.byte_end)?;
        dict.set_item("position", self.position)?;
        dict.set_item("word_id", self.word_id)?;
        dict.set_item("is_unknown", self.is_unknown)?;
        dict.set_item("details", &self.details)?;
        match &self.dependency {
            Some(dependency) => dict.set_item("dependency", dependency.to_dict(py)?)?,
            None => dict.set_item("dependency", py.None())?,
        }
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
    /// Builds a `PyToken` from a lindera-binding [`TokenView`](lindera_binding::TokenView).
    pub fn from_view(view: lindera_binding::TokenView) -> Self {
        Self {
            surface: view.surface,
            byte_start: view.byte_start,
            byte_end: view.byte_end,
            position: view.position,
            word_id: view.word_id,
            is_unknown: view.is_unknown,
            details: view.details,
            dependency: view.dependency.map(PyTokenDependency::from_view),
        }
    }

    /// Builds a `PyToken` from a `lindera` token via [`TokenView`](lindera_binding::TokenView).
    pub fn from_token(token: Token) -> Self {
        Self::from_view(lindera_binding::TokenView::from_token(token))
    }
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "token")?;
    m.add_class::<PyToken>()?;
    m.add_class::<PyTokenDependency>()?;
    parent_module.add_submodule(&m)?;
    Ok(())
}
