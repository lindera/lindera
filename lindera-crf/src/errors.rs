//! Definition of errors.

use core::fmt;

#[cfg(feature = "std")]
use std::error::Error;

/// Error used when the argument is invalid.
#[derive(Debug)]
pub struct InvalidArgumentError {
    msg: &'static str,
}

impl fmt::Display for InvalidArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InvalidArgumentError: {}", self.msg)
    }
}

#[cfg(feature = "std")]
impl Error for InvalidArgumentError {}

/// Error used when the model becomes too large.
#[derive(Debug)]
pub struct ModelScaleError {
    msg: &'static str,
}

impl fmt::Display for ModelScaleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ModelScaleError: {}", self.msg)
    }
}

#[cfg(feature = "std")]
impl Error for ModelScaleError {}

/// The error type for Rucrf.
#[derive(Debug)]
pub enum RucrfError {
    /// Error variant for [`InvalidArgumentError`].
    InvalidArgument(InvalidArgumentError),

    /// Error variant for [`ModelScaleError`].
    ModelScale(ModelScaleError),
}

impl RucrfError {
    /// Creates a new [`InvalidArgumentError`].
    pub(crate) const fn invalid_argument(msg: &'static str) -> Self {
        Self::InvalidArgument(InvalidArgumentError { msg })
    }

    /// Creates a new [`ModelScaleError`].
    pub(crate) const fn model_scale(msg: &'static str) -> Self {
        Self::ModelScale(ModelScaleError { msg })
    }
}

impl fmt::Display for RucrfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidArgument(e) => e.fmt(f),
            Self::ModelScale(e) => e.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl Error for RucrfError {}

/// A specialized Result type.
pub type Result<T, E = RucrfError> = core::result::Result<T, E>;
