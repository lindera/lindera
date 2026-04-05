//! Error types for Lindera operations.
//!
//! This module provides error conversion from Lindera errors to PHP exceptions.

use ext_php_rs::exception::PhpException;
use ext_php_rs::zend::ce;

/// Converts a displayable error into a PHP exception.
///
/// # Arguments
///
/// * `err` - Error implementing Display trait.
///
/// # Returns
///
/// A `PhpException` wrapping the error message.
pub fn lindera_err(err: impl std::fmt::Display) -> PhpException {
    PhpException::new(format!("Lindera error: {err}"), 0, ce::exception())
}

/// Converts a displayable error into a PHP ValueError exception.
///
/// # Arguments
///
/// * `err` - Error implementing Display trait.
///
/// # Returns
///
/// A `PhpException` with ValueError class.
pub fn lindera_value_err(err: impl std::fmt::Display) -> PhpException {
    PhpException::new(format!("{err}"), 0, ce::value_error())
}
