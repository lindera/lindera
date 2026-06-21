//! Shared, FFI-independent error type for the language bindings.
//!
//! Each binding historically defined its own error plumbing (a `PyException`
//! wrapper, a `napi::Error` helper, a magnus `Error`, a `PhpException`, a
//! `JsValue`), all of them message-only. This module provides a single
//! [`CoreError`] carrying an [`ErrorKind`] category plus a message, so each
//! binding can keep just one `From<CoreError>` conversion into its native
//! exception type.

use std::fmt;

use lindera::error::{LinderaError, LinderaErrorKind};

/// Category of a binding-facing error.
///
/// Lets each binding map a [`CoreError`] onto the most appropriate native
/// exception (for example, an [`ErrorKind::InvalidArgument`] can become a
/// `ValueError` rather than a generic exception).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The caller supplied an invalid argument or value.
    InvalidArgument,
    /// A dictionary could not be found, loaded, or resolved.
    Dictionary,
    /// A tokenizer or other component failed to build.
    Build,
    /// An input/output operation failed.
    Io,
    /// Data could not be parsed, decoded, or (de)serialized.
    Parse,
    /// A schema or record failed validation.
    Validation,
    /// A runtime failure that does not fit a more specific category.
    Runtime,
}

impl ErrorKind {
    /// Returns a stable, lowercase identifier for the kind, suitable for
    /// logging or exposing to the host language.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::InvalidArgument => "invalid_argument",
            ErrorKind::Dictionary => "dictionary",
            ErrorKind::Build => "build",
            ErrorKind::Io => "io",
            ErrorKind::Parse => "parse",
            ErrorKind::Validation => "validation",
            ErrorKind::Runtime => "runtime",
        }
    }
}

impl fmt::Display for ErrorKind {
    /// Writes the lowercase identifier returned by [`ErrorKind::as_str`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An FFI-independent error shared by all Lindera language bindings.
///
/// Carries an [`ErrorKind`] category plus a human-readable message. Each
/// binding implements a single `From<CoreError>` to map it onto its native
/// exception type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreError {
    /// The error category.
    kind: ErrorKind,
    /// The human-readable error message.
    message: String,
}

impl CoreError {
    /// Creates a new error with the given kind and message.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Creates an [`ErrorKind::InvalidArgument`] error.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidArgument, message)
    }

    /// Creates an [`ErrorKind::Dictionary`] error.
    pub fn dictionary(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Dictionary, message)
    }

    /// Creates an [`ErrorKind::Build`] error.
    pub fn build(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Build, message)
    }

    /// Creates an [`ErrorKind::Validation`] error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    /// Creates an [`ErrorKind::Runtime`] error.
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Runtime, message)
    }

    /// Returns the error category.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the human-readable error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CoreError {
    /// Writes the error message (the kind is available via [`CoreError::kind`]).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CoreError {}

impl From<LinderaError> for CoreError {
    /// Maps a [`lindera::error::LinderaError`] onto a [`CoreError`], translating
    /// the [`LinderaErrorKind`] into the binding-facing [`ErrorKind`].
    fn from(err: LinderaError) -> Self {
        let kind = match err.kind() {
            LinderaErrorKind::Args | LinderaErrorKind::Mode => ErrorKind::InvalidArgument,
            LinderaErrorKind::Dictionary | LinderaErrorKind::NotFound => ErrorKind::Dictionary,
            LinderaErrorKind::Build => ErrorKind::Build,
            LinderaErrorKind::Io => ErrorKind::Io,
            LinderaErrorKind::Content
            | LinderaErrorKind::Decode
            | LinderaErrorKind::Deserialize
            | LinderaErrorKind::Serialize
            | LinderaErrorKind::Parse => ErrorKind::Parse,
            LinderaErrorKind::FeatureDisabled => ErrorKind::Runtime,
        };
        CoreError::new(kind, err.to_string())
    }
}

/// A convenient `Result` alias for binding-core operations.
pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_exposes_kind_and_message() {
        let err = CoreError::new(ErrorKind::Build, "boom");
        assert_eq!(err.kind(), ErrorKind::Build);
        assert_eq!(err.message(), "boom");
        assert_eq!(err.to_string(), "boom");
    }

    #[test]
    fn convenience_constructors_set_kind() {
        assert_eq!(
            CoreError::invalid_argument("x").kind(),
            ErrorKind::InvalidArgument
        );
        assert_eq!(CoreError::dictionary("x").kind(), ErrorKind::Dictionary);
        assert_eq!(CoreError::validation("x").kind(), ErrorKind::Validation);
        assert_eq!(CoreError::runtime("x").kind(), ErrorKind::Runtime);
    }

    #[test]
    fn kind_as_str_is_stable() {
        assert_eq!(ErrorKind::InvalidArgument.as_str(), "invalid_argument");
        assert_eq!(ErrorKind::Dictionary.to_string(), "dictionary");
    }

    #[test]
    fn from_lindera_error_maps_kind() {
        let parse = LinderaErrorKind::Parse.with_error(std::io::Error::other("bad"));
        assert_eq!(CoreError::from(parse).kind(), ErrorKind::Parse);

        let args = LinderaErrorKind::Args.with_error(std::io::Error::other("bad arg"));
        assert_eq!(CoreError::from(args).kind(), ErrorKind::InvalidArgument);

        let notfound = LinderaErrorKind::NotFound.with_error(std::io::Error::other("missing"));
        assert_eq!(CoreError::from(notfound).kind(), ErrorKind::Dictionary);
    }
}
