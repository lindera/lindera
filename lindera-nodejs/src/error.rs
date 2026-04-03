//! Error handling utilities for Lindera Node.js bindings.
//!
//! Provides helper functions to convert Rust errors into napi-rs errors
//! that are thrown as JavaScript exceptions.

use std::fmt::Display;

use napi::Status;

/// Converts any displayable error into a napi error.
///
/// # Arguments
///
/// * `err` - Any error type that implements `Display`.
///
/// # Returns
///
/// A `napi::Error` with `GenericFailure` status and the error message.
pub fn to_napi_error(err: impl Display) -> napi::Error {
    napi::Error::new(Status::GenericFailure, format!("{err}"))
}
