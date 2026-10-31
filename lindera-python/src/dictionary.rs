//! Dictionary management for morphological analysis.
//!
//! This module provides functionality for building, loading, and managing dictionaries
//! used in morphological analysis.
//!
//! # Dictionary Types
//!
//! - **Dictionary**: Main dictionary for morphological analysis
//! - **UserDictionary**: Custom user-defined dictionary for additional words
//!
//! # Examples
//!
//! ```python
//! import lindera
//!
//! # Load a pre-built dictionary
//! dictionary = lindera.load_dictionary("ipadic")
//!
//! # Build a custom dictionary
//! metadata = lindera.Metadata()
//! lindera.build_dictionary("/path/to/input", "/path/to/output", metadata)
//!
//! # Build a user dictionary for a system dictionary
//! lindera.build_user_dictionary_for("user.csv", "/path/to/user.bin", dictionary)
//! ```

use std::path::Path;

use pyo3::{exceptions::PyValueError, prelude::*};

use lindera::dictionary::{
    Dictionary, DictionaryBuilder, Metadata, UserDictionary,
    load_dictionary as lindera_load_dictionary,
    load_user_dictionary as lindera_load_user_dictionary,
    load_user_dictionary_for as lindera_load_user_dictionary_for,
};

use crate::metadata::PyMetadata;

/// A morphological analysis dictionary.
///
/// Contains the data structures needed for tokenization and morphological analysis.
///
/// # Examples
///
/// ```python
/// # Load a dictionary
/// dictionary = lindera.load_dictionary("ipadic")
///
/// # Access metadata
/// print(dictionary.metadata_name())
/// print(dictionary.metadata_encoding())
/// ```
#[pyclass(name = "Dictionary", from_py_object)]
#[derive(Clone)]
pub struct PyDictionary {
    pub inner: Dictionary,
}

#[pymethods]
impl PyDictionary {
    /// Returns the name of the dictionary metadata.
    pub fn metadata_name(&self) -> String {
        self.inner.metadata.name.clone()
    }

    /// Returns the character encoding of the dictionary.
    pub fn metadata_encoding(&self) -> String {
        self.inner.metadata.encoding.clone()
    }

    /// Returns the full metadata object of the dictionary.
    pub fn metadata(&self) -> PyMetadata {
        PyMetadata::from((*self.inner.metadata).clone())
    }

    fn __str__(&self) -> String {
        "Dictionary".to_string()
    }

    fn __repr__(&self) -> String {
        "Dictionary()".to_string()
    }
}

impl PyDictionary {
    // Internal helper function to create PyDictionary from Lindera Dictionary
    pub fn new(dictionary: Dictionary) -> Self {
        Self { inner: dictionary }
    }
}

/// A user-defined dictionary for custom words.
///
/// User dictionaries allow you to add custom words and their morphological features
/// that are not present in the main dictionary.
///
/// # Examples
///
/// ```python
/// # Build a user dictionary for a system dictionary
/// dictionary = lindera.load_dictionary("embedded://ipadic")
/// lindera.build_user_dictionary_for("user.csv", "/path/to/user.bin", dictionary)
///
/// # Load it
/// user_dict = lindera.load_user_dictionary_for("/path/to/user.bin", dictionary)
/// ```
#[pyclass(name = "UserDictionary", from_py_object)]
#[derive(Clone)]
pub struct PyUserDictionary {
    pub inner: UserDictionary,
}

#[pymethods]
impl PyUserDictionary {
    fn __str__(&self) -> String {
        "UserDictionary".to_string()
    }

    fn __repr__(&self) -> String {
        "UserDictionary()".to_string()
    }
}

impl PyUserDictionary {
    // Internal helper function to create PyUserDictionary from Lindera UserDictionary
    pub fn new(user_dictionary: UserDictionary) -> Self {
        Self {
            inner: user_dictionary,
        }
    }
}

/// Builds a dictionary from source files.
///
/// When the metadata enables `connection_id_mapping`, as the `metadata.json`
/// of ko-dic and UniDic does, the build relabels the connection matrix's
/// context IDs by access frequency: by the counts of `context_id_freq`, or,
/// without it, by the count of dictionary entries. The relabelling changes
/// the speed of tokenization, never its result.
///
/// # Arguments
///
/// * `input_dir` - Directory containing dictionary source files.
/// * `output_dir` - Directory where the built dictionary will be saved.
/// * `metadata` - Metadata configuration for the dictionary.
/// * `context_id_freq` - Optional path of a context-ID frequency histogram,
///   such as the `context_id_freq.txt` of a dictionary crate, as `lindera
///   build --context-id-freq` takes it. Read only with
///   `connection_id_mapping`.
///
/// # Errors
///
/// Returns an error if the input directory doesn't exist or if the build
/// fails, including when the histogram cannot be read.
///
/// # Examples
///
/// ```python
/// metadata = lindera.Metadata(name="custom", encoding="UTF-8")
/// lindera.build_dictionary("/path/to/input", "/path/to/output", metadata)
///
/// metadata = lindera.Metadata.from_json_file("lindera-ko-dic/metadata.json")
/// lindera.build_dictionary(
///     "/path/to/mecab-ko-dic",
///     "/path/to/output",
///     metadata,
///     context_id_freq="lindera-ko-dic/context_id_freq.txt",
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (input_dir, output_dir, metadata, context_id_freq=None))]
pub fn build_dictionary(
    input_dir: &str,
    output_dir: &str,
    metadata: PyMetadata,
    context_id_freq: Option<&str>,
) -> PyResult<()> {
    let input_path = Path::new(input_dir);
    let output_path = Path::new(output_dir);

    if !input_path.exists() {
        return Err(PyValueError::new_err(format!(
            "Input directory does not exist: {input_dir}"
        )));
    }

    lindera_binding::dictionary::build_dictionary(
        input_path,
        output_path,
        metadata.into(),
        context_id_freq.map(Path::new),
    )
    .map_err(|e| PyValueError::new_err(format!("Failed to build dictionary: {e}")))?;

    Ok(())
}

/// Builds a user dictionary from a CSV file.
///
/// The rows' context IDs are not checked here; they are checked when the
/// user dictionary is combined with a system dictionary. To check them as
/// the CSV is built, use [`build_user_dictionary_for`] with the system
/// dictionary.
///
/// # Arguments
///
/// * `_kind` - Dictionary kind (currently unused, reserved for future use).
/// * `input_file` - Path to the CSV file containing user dictionary entries.
/// * `output_dir` - Path of the file to write the built user dictionary to,
///   not a directory despite its name. The user dictionary loaders choose
///   how to load a file by its extension, so end it with `.bin`.
/// * `metadata` - Optional metadata configuration. If None, default values are used.
///
/// # CSV Format
///
/// The CSV file should contain entries in the format specified by the dictionary schema.
/// Typically: surface,reading,pronunciation
///
/// # Errors
///
/// Returns an error if the input file doesn't exist or if the build fails.
///
/// # Examples
///
/// ```python
/// # Build with default metadata
/// lindera.build_user_dictionary("ipadic", "user.csv", "/path/to/user.bin")
///
/// # Build with custom metadata
/// metadata = lindera.Metadata()
/// lindera.build_user_dictionary("ipadic", "user.csv", "/path/to/user.bin", metadata)
/// ```
#[pyfunction]
#[pyo3(signature = (_kind, input_file, output_dir, metadata=None))]
pub fn build_user_dictionary(
    _kind: &str,
    input_file: &str,
    output_dir: &str,
    metadata: Option<crate::metadata::PyMetadata>,
) -> PyResult<()> {
    let input_path = Path::new(input_file);
    let output_path = Path::new(output_dir);

    if !input_path.exists() {
        return Err(PyValueError::new_err(format!(
            "Input file does not exist: {input_file}"
        )));
    }

    // Use provided metadata or create default
    let meta = match metadata {
        Some(py_metadata) => {
            let lindera_meta: Metadata = py_metadata.into();
            lindera_meta
        }
        None => Metadata::default(),
    };

    let builder = DictionaryBuilder::new(meta);

    // Build user dictionary from CSV
    builder
        .build_user_dictionary(input_path, output_path)
        .map_err(|e| PyValueError::new_err(format!("Failed to build user dictionary: {e}")))?;

    Ok(())
}

/// Builds a user dictionary from a CSV file for a loaded system dictionary.
///
/// The CSV is built with the system dictionary's metadata, and every row's
/// context IDs are checked against its connection matrix, so a row outside
/// the matrix fails with an error that names its CSV line and surface, and
/// no file is written.
///
/// # Arguments
///
/// * `input_file` - Path to the CSV file containing user dictionary entries.
/// * `output_file` - Path of the file to write the built user dictionary to.
///   The user dictionary loaders choose how to load a file by its extension,
///   so end it with `.bin`.
/// * `dictionary` - The system dictionary the user dictionary is built for.
///
/// # Errors
///
/// Returns an error if the CSV cannot be read or built, if a row has a
/// context ID outside the system dictionary's connection matrix, or if the
/// file cannot be written.
///
/// # Examples
///
/// ```python
/// dictionary = lindera.load_dictionary("embedded://ipadic")
/// lindera.build_user_dictionary_for("user.csv", "/path/to/user.bin", dictionary)
/// user_dict = lindera.load_user_dictionary_for("/path/to/user.bin", dictionary)
/// ```
#[pyfunction]
#[pyo3(signature = (input_file, output_file, dictionary))]
pub fn build_user_dictionary_for(
    input_file: &str,
    output_file: &str,
    dictionary: PyDictionary,
) -> PyResult<()> {
    lindera_binding::dictionary::build_user_dictionary_for(
        Path::new(input_file),
        Path::new(output_file),
        &dictionary.inner,
    )
    .map_err(|e| PyValueError::new_err(format!("Failed to build user dictionary: {e}")))
}

/// Loads a dictionary from the specified URI.
///
/// # Arguments
///
/// * `uri` - URI to the dictionary. Can be a file path or embedded dictionary name.
///
/// # Supported URIs
///
/// - File paths: `/path/to/dictionary`
/// - Embedded dictionaries: `ipadic`, `unidic`, `ko-dic`, `cc-cedict`
///
/// # Returns
///
/// A loaded `Dictionary` object.
///
/// # Errors
///
/// Returns an error if the dictionary cannot be loaded from the specified URI.
///
/// # Examples
///
/// ```python
/// # Load an embedded dictionary
/// dict = lindera.load_dictionary("ipadic")
///
/// # Load from file path
/// dict = lindera.load_dictionary("/path/to/dictionary")
/// ```
#[pyfunction]
#[pyo3(signature = (uri))]
pub fn load_dictionary(uri: &str) -> PyResult<PyDictionary> {
    lindera_load_dictionary(uri)
        .map_err(|e| PyValueError::new_err(format!("Failed to load dictionary from '{uri}': {e}")))
        .map(PyDictionary::new)
}

/// Loads a user dictionary from the specified URI.
///
/// # Arguments
///
/// * `uri` - URI to the user dictionary directory.
/// * `metadata` - Metadata configuration for the user dictionary.
///
/// # Returns
///
/// A loaded `UserDictionary` object.
///
/// # Errors
///
/// Returns an error if the user dictionary cannot be loaded.
///
/// # Examples
///
/// ```python
/// metadata = lindera.Metadata()
/// user_dict = lindera.load_user_dictionary("/path/to/user_dict", metadata)
/// ```
#[pyfunction]
#[pyo3(signature = (uri, metadata))]
pub fn load_user_dictionary(uri: &str, metadata: PyMetadata) -> PyResult<PyUserDictionary> {
    let meta: Metadata = metadata.into();
    lindera_load_user_dictionary(uri, &meta)
        .map_err(|e| {
            PyValueError::new_err(format!("Failed to load user dictionary from '{uri}': {e}"))
        })
        .map(PyUserDictionary::new)
}

/// Loads a user dictionary for a loaded system dictionary.
///
/// A `.csv` is built with the system dictionary's metadata and checked
/// against its connection matrix as it is built, so a row whose context ID
/// lies outside the matrix fails with an error that names its CSV line and
/// surface. A `.bin` is loaded as it was built; the `Tokenizer` checks it
/// when it is built with the system dictionary.
///
/// # Arguments
///
/// * `uri` - URI or path of the `.csv` or `.bin` user dictionary.
/// * `dictionary` - The system dictionary the user dictionary is used with.
///
/// # Returns
///
/// A loaded `UserDictionary` object.
///
/// # Errors
///
/// Returns an error if the user dictionary cannot be loaded, or if a `.csv`
/// row has a context ID outside the system dictionary's connection matrix.
///
/// # Examples
///
/// ```python
/// dictionary = lindera.load_dictionary("embedded://ipadic")
/// user_dict = lindera.load_user_dictionary_for("/path/to/user_dict.csv", dictionary)
/// ```
#[pyfunction]
#[pyo3(signature = (uri, dictionary))]
pub fn load_user_dictionary_for(uri: &str, dictionary: PyDictionary) -> PyResult<PyUserDictionary> {
    lindera_load_user_dictionary_for(uri, &dictionary.inner)
        .map_err(|e| {
            PyValueError::new_err(format!("Failed to load user dictionary from '{uri}': {e}"))
        })
        .map(PyUserDictionary::new)
}

pub fn register(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent_module.py();
    let m = PyModule::new(py, "dictionary")?;
    m.add_class::<PyDictionary>()?;
    m.add_class::<PyUserDictionary>()?;
    m.add_function(wrap_pyfunction!(build_dictionary, &m)?)?;
    m.add_function(wrap_pyfunction!(build_user_dictionary, &m)?)?;
    m.add_function(wrap_pyfunction!(build_user_dictionary_for, &m)?)?;
    m.add_function(wrap_pyfunction!(load_dictionary, &m)?)?;
    m.add_function(wrap_pyfunction!(load_user_dictionary, &m)?)?;
    m.add_function(wrap_pyfunction!(load_user_dictionary_for, &m)?)?;
    parent_module.add_submodule(&m)?;
    Ok(())
}
