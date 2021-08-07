use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum BuildErrorKind {
    Content,
    Decode,
    Io,
    Parse,
    Serialize,
}

impl BuildErrorKind {
    pub fn with_error<E>(self, source: E) -> BuildError
    where
        anyhow::Error: From<E>,
    {
        BuildError {
            kind: self,
            source: From::from(source),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("BuildError(kind={kind:?}, source={source})")]
pub struct BuildError {
    pub kind: BuildErrorKind,
    #[source]
    source: anyhow::Error,
}

impl BuildError {
    pub fn add_context<C>(self, ctx: C) -> Self
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        BuildError {
            kind: self.kind,
            source: self.source.context(ctx),
        }
    }

    pub fn kind(&self) -> BuildErrorKind {
        self.kind
    }
}

pub type BuildResult<T> = Result<T, BuildError>;
