//! Dictionary management for morphological analysis.
//!
//! This module provides functionality for building, loading, and managing dictionaries
//! used in morphological analysis.

use std::path::Path;

use magnus::prelude::*;
use magnus::scan_args::{get_kwargs, scan_args};
use magnus::{Error, RHash, Ruby, Value, function, method};

use lindera::dictionary::{
    Dictionary, DictionaryBuilder, Metadata, UserDictionary,
    load_dictionary as lindera_load_dictionary,
    load_user_dictionary as lindera_load_user_dictionary,
    load_user_dictionary_for as lindera_load_user_dictionary_for,
};

use crate::error::to_magnus_error;
use crate::metadata::RbMetadata;

/// A morphological analysis dictionary.
///
/// Contains the data structures needed for tokenization and morphological analysis.
#[magnus::wrap(class = "Lindera::Dictionary", free_immediately, size)]
#[derive(Clone)]
pub struct RbDictionary {
    /// Inner Lindera dictionary.
    pub inner: Dictionary,
}

impl RbDictionary {
    /// Returns the name of the dictionary metadata.
    ///
    /// # Returns
    ///
    /// The dictionary metadata name.
    fn metadata_name(&self) -> String {
        self.inner.metadata.name.clone()
    }

    /// Returns the character encoding of the dictionary.
    ///
    /// # Returns
    ///
    /// The dictionary encoding string.
    fn metadata_encoding(&self) -> String {
        self.inner.metadata.encoding.clone()
    }

    /// Returns the full metadata object.
    ///
    /// # Returns
    ///
    /// The dictionary metadata.
    fn metadata(&self) -> RbMetadata {
        RbMetadata::from((*self.inner.metadata).clone())
    }

    /// Returns the string representation.
    fn to_s(&self) -> String {
        "Dictionary".to_string()
    }

    /// Returns the inspect representation.
    fn inspect(&self) -> String {
        format!(
            "#<Lindera::Dictionary: name='{}'>",
            self.inner.metadata.name
        )
    }
}

/// A user-defined dictionary for custom words.
///
/// User dictionaries allow you to add custom words and their morphological features
/// that are not present in the main dictionary.
#[magnus::wrap(class = "Lindera::UserDictionary", free_immediately, size)]
#[derive(Clone)]
pub struct RbUserDictionary {
    /// Inner Lindera user dictionary.
    pub inner: UserDictionary,
}

impl RbUserDictionary {
    /// Returns the string representation.
    fn to_s(&self) -> String {
        "UserDictionary".to_string()
    }

    /// Returns the inspect representation.
    fn inspect(&self) -> String {
        "UserDictionary()".to_string()
    }
}

/// Loads a dictionary from the specified URI.
///
/// # Arguments
///
/// * `uri` - URI to the dictionary. Can be a file path or embedded dictionary name.
///
/// # Returns
///
/// A loaded `RbDictionary` object.
fn load_dictionary(uri: String) -> Result<RbDictionary, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not initialized");
    lindera_load_dictionary(&uri)
        .map_err(|e| {
            to_magnus_error(
                &ruby,
                format!("Failed to load dictionary from '{uri}': {e}"),
            )
        })
        .map(|d| RbDictionary { inner: d })
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
/// A loaded `RbUserDictionary` object.
fn load_user_dictionary(uri: String, metadata: &RbMetadata) -> Result<RbUserDictionary, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not initialized");
    let meta: Metadata = metadata.clone().into();
    lindera_load_user_dictionary(&uri, &meta)
        .map_err(|e| {
            to_magnus_error(
                &ruby,
                format!("Failed to load user dictionary from '{uri}': {e}"),
            )
        })
        .map(|d| RbUserDictionary { inner: d })
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
/// * `ruby` - Ruby runtime handle (passed by magnus, not by the caller).
/// * `uri` - URI or path of the `.csv` or `.bin` user dictionary.
/// * `dictionary` - The system dictionary the user dictionary is used with.
///
/// # Returns
///
/// A loaded `RbUserDictionary` object.
fn load_user_dictionary_for(
    ruby: &Ruby,
    uri: String,
    dictionary: &RbDictionary,
) -> Result<RbUserDictionary, Error> {
    lindera_load_user_dictionary_for(&uri, &dictionary.inner)
        .map_err(|e| {
            to_magnus_error(
                ruby,
                format!("Failed to load user dictionary from '{uri}': {e}"),
            )
        })
        .map(|d| RbUserDictionary { inner: d })
}

/// Builds a dictionary from source files.
///
/// Ruby signature: `Lindera.build_dictionary(input_dir, output_dir, metadata,
/// context_id_freq: nil)`.
///
/// When the metadata enables `connection_id_mapping`, as the `metadata.json`
/// of ko-dic and UniDic does, the build relabels the connection matrix's
/// context IDs by access frequency: by the counts of `context_id_freq`, or,
/// without it, by the count of dictionary entries. The relabelling changes
/// the speed of tokenization, never its result.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `args` - The Ruby arguments:
///   * `input_dir` - Directory containing dictionary source files.
///   * `output_dir` - Directory where the built dictionary will be saved.
///   * `metadata` - Metadata configuration for the dictionary.
///   * `context_id_freq:` - Path of a context-ID frequency histogram, such
///     as the `context_id_freq.txt` of a dictionary crate, as `lindera
///     build --context-id-freq` takes it. `nil` or omitted means none. Read
///     only with `connection_id_mapping`.
///
/// # Returns
///
/// `Ok(())` on success, an `ArgumentError` for a wrong number of arguments,
/// an unknown keyword or a missing input directory, a `TypeError` for an
/// argument of the wrong type, or a `RuntimeError` when the build fails,
/// including when the histogram cannot be read.
fn build_dictionary(ruby: &Ruby, args: &[Value]) -> Result<(), Error> {
    let args = scan_args::<(String, String, &RbMetadata), (), (), (), RHash, ()>(args)?;
    let (input_dir, output_dir, metadata) = args.required;
    let keywords = get_kwargs::<_, (), (Option<Option<String>>,), ()>(
        args.keywords,
        &[],
        &["context_id_freq"],
    )?;
    let (context_id_freq,) = keywords.optional;
    let context_id_freq = context_id_freq.flatten();

    let input_path = Path::new(&input_dir);
    let output_path = Path::new(&output_dir);

    if !input_path.exists() {
        return Err(Error::new(
            ruby.exception_arg_error(),
            format!("Input directory does not exist: {input_dir}"),
        ));
    }

    lindera_binding::dictionary::build_dictionary(
        input_path,
        output_path,
        metadata.clone().into(),
        context_id_freq.as_deref().map(Path::new),
    )
    .map_err(|e| to_magnus_error(ruby, format!("Failed to build dictionary: {e}")))?;

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
/// * `_kind` - Dictionary kind (reserved for future use).
/// * `input_file` - Path to the CSV file.
/// * `output_dir` - Path of the file to write the built user dictionary to,
///   not a directory despite its name. The user dictionary loaders choose
///   how to load a file by its extension, so end it with `.bin`.
/// * `metadata` - Optional metadata configuration.
fn build_user_dictionary(
    _kind: String,
    input_file: String,
    output_dir: String,
    metadata: Option<&RbMetadata>,
) -> Result<(), Error> {
    let ruby = Ruby::get().expect("Ruby runtime not initialized");
    let input_path = Path::new(&input_file);
    let output_path = Path::new(&output_dir);

    if !input_path.exists() {
        return Err(Error::new(
            ruby.exception_arg_error(),
            format!("Input file does not exist: {input_file}"),
        ));
    }

    let meta = match metadata {
        Some(m) => m.clone().into(),
        None => Metadata::default(),
    };

    let builder = DictionaryBuilder::new(meta);
    builder
        .build_user_dictionary(input_path, output_path)
        .map_err(|e| to_magnus_error(&ruby, format!("Failed to build user dictionary: {e}")))?;

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
/// * `ruby` - Ruby runtime handle (passed by magnus, not by the caller).
/// * `input_file` - Path to the CSV file.
/// * `output_file` - Path of the file to write the built user dictionary to.
///   The user dictionary loaders choose how to load a file by its extension,
///   so end it with `.bin`.
/// * `dictionary` - The system dictionary the user dictionary is built for.
///
/// # Returns
///
/// `Ok(())` once the file is written, or a Ruby `RuntimeError` if the CSV
/// cannot be read or built, a row has a context ID outside the system
/// dictionary's connection matrix, or the file cannot be written.
fn build_user_dictionary_for(
    ruby: &Ruby,
    input_file: String,
    output_file: String,
    dictionary: &RbDictionary,
) -> Result<(), Error> {
    lindera_binding::dictionary::build_user_dictionary_for(
        Path::new(&input_file),
        Path::new(&output_file),
        &dictionary.inner,
    )
    .map_err(|e| to_magnus_error(ruby, format!("Failed to build user dictionary: {e}")))
}

/// Defines Dictionary, UserDictionary classes and module functions in the given Ruby module.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `module` - Parent Ruby module.
///
/// # Returns
///
/// `Ok(())` on success, or a Magnus `Error` on failure.
pub fn define(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    let dict_class = module.define_class("Dictionary", ruby.class_object())?;
    dict_class.define_method("metadata_name", method!(RbDictionary::metadata_name, 0))?;
    dict_class.define_method(
        "metadata_encoding",
        method!(RbDictionary::metadata_encoding, 0),
    )?;
    dict_class.define_method("metadata", method!(RbDictionary::metadata, 0))?;
    dict_class.define_method("to_s", method!(RbDictionary::to_s, 0))?;
    dict_class.define_method("inspect", method!(RbDictionary::inspect, 0))?;

    let user_dict_class = module.define_class("UserDictionary", ruby.class_object())?;
    user_dict_class.define_method("to_s", method!(RbUserDictionary::to_s, 0))?;
    user_dict_class.define_method("inspect", method!(RbUserDictionary::inspect, 0))?;

    module.define_module_function("load_dictionary", function!(load_dictionary, 1))?;
    module.define_module_function("load_user_dictionary", function!(load_user_dictionary, 2))?;
    module.define_module_function(
        "load_user_dictionary_for",
        function!(load_user_dictionary_for, 2),
    )?;
    module.define_module_function("build_dictionary", function!(build_dictionary, -1))?;
    module.define_module_function("build_user_dictionary", function!(build_user_dictionary, 4))?;
    module.define_module_function(
        "build_user_dictionary_for",
        function!(build_user_dictionary_for, 3),
    )?;

    Ok(())
}
