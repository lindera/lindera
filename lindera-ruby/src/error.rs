//! Error types for Lindera operations.
//!
//! This module provides error types used throughout the Lindera Ruby bindings.

use magnus::{Error, Ruby};

/// Converts a Lindera error message into a Magnus Ruby error.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `msg` - Error message string.
///
/// # Returns
///
/// A Magnus `Error` wrapping a Ruby `RuntimeError`.
pub fn to_magnus_error(ruby: &Ruby, msg: String) -> Error {
    Error::new(ruby.exception_runtime_error(), msg)
}
