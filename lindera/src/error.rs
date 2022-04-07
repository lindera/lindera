use lindera_core::error::{
    LinderaError as LinderaCoreError, LinderaErrorKind as LinderaCoreErrorKind,
};

pub type LinderaErrorKind = LinderaCoreErrorKind;

pub type LinderaError = LinderaCoreError;
