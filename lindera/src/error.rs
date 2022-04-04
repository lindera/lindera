use std::fmt;

use serde::{Deserialize, Serialize};

use lindera_core::error::{
    LinderaError as LinderaCoreError, LinderaErrorKind as LinderaCoreErrorKind,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum LinderaErrorKind {
    Args,
    Content,
    Decode,
    Deserialize,
    Io,
    Parse,
    Serialize,
    Compress,
    DictionaryNotFound,
    DictionaryLoadError,
    DictionaryBuildError,
    DictionaryTypeError,
    UserDictionaryTypeError,
    ModeError,
}

impl From<LinderaCoreError> for LinderaErrorKind {
    fn from(err: LinderaCoreError) -> Self {
        match err.kind() {
            LinderaCoreErrorKind::Args => LinderaErrorKind::Args,
            LinderaCoreErrorKind::Content => LinderaErrorKind::Content,
            LinderaCoreErrorKind::Decode => LinderaErrorKind::Decode,
            LinderaCoreErrorKind::Deserialize => LinderaErrorKind::Deserialize,
            LinderaCoreErrorKind::Io => LinderaErrorKind::Io,
            LinderaCoreErrorKind::Parse => LinderaErrorKind::Parse,
            LinderaCoreErrorKind::Serialize => LinderaErrorKind::Serialize,
            LinderaCoreErrorKind::Compress => LinderaErrorKind::Compress,
            LinderaCoreErrorKind::DictionaryNotFound => LinderaErrorKind::DictionaryNotFound,
            LinderaCoreErrorKind::DictionaryLoadError => LinderaErrorKind::DictionaryLoadError,
            LinderaCoreErrorKind::DictionaryBuildError => LinderaErrorKind::DictionaryBuildError,
            LinderaCoreErrorKind::DictionaryTypeError => LinderaErrorKind::DictionaryTypeError,
            LinderaCoreErrorKind::UserDictionaryTypeError => {
                LinderaErrorKind::UserDictionaryTypeError
            }
            LinderaCoreErrorKind::ModeError => LinderaErrorKind::ModeError,
        }
    }
}

impl LinderaErrorKind {
    pub fn with_error<E>(self, source: E) -> LinderaError
    where
        anyhow::Error: From<E>,
    {
        LinderaError {
            kind: self,
            source: From::from(source),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("LinderaError(kind={kind:?}, source={source})")]
pub struct LinderaError {
    pub kind: LinderaErrorKind,
    #[source]
    source: anyhow::Error,
}

impl LinderaError {
    pub fn add_context<C>(self, ctx: C) -> Self
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        LinderaError {
            kind: self.kind,
            source: self.source.context(ctx),
        }
    }

    pub fn kind(&self) -> LinderaErrorKind {
        self.kind
    }
}
