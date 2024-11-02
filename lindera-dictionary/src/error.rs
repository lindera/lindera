use std::fmt;

use serde::{Deserialize, Serialize};

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
    Decompress,
    NotFound,
    Load,
    Build,
    Dictionary,
    Source,
    Mode,
    Token,
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
