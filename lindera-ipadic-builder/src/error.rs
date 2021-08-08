use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum BuildDictionaryErrorKind {
    Content,
    Decode,
    Io,
    Parse,
    Serialize,
}

impl BuildDictionaryErrorKind {
    pub fn with_error<E>(self, source: E) -> BuildDictionaryError
    where
        anyhow::Error: From<E>,
    {
        BuildDictionaryError {
            kind: self,
            source: From::from(source),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("BuildError(kind={kind:?}, source={source})")]
pub struct BuildDictionaryError {
    pub kind: BuildDictionaryErrorKind,
    #[source]
    source: anyhow::Error,
}

impl BuildDictionaryError {
    pub fn add_context<C>(self, ctx: C) -> Self
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        BuildDictionaryError {
            kind: self.kind,
            source: self.source.context(ctx),
        }
    }

    pub fn kind(&self) -> BuildDictionaryErrorKind {
        self.kind
    }
}

pub type BuildDictionaryResult<T> = Result<T, BuildDictionaryError>;
