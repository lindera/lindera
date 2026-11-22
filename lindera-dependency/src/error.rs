//! The error type of this crate.

use std::fmt;
use std::io;

/// The result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// An error of this crate.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The input is not in the lattice format.
    Format {
        /// The number of the line, from 1.
        line: usize,
        /// What is wrong with the line.
        message: String,
    },
    /// A sentence is not one the lattice format can hold: its bunsetsu do not
    /// cover its morphemes, a head or a position is out of range, or a
    /// string holds a tab or a line break.
    Structure(String),
    /// Reading or writing failed.
    Io(io::Error),
    /// The input is not a CoNLL-U file the corpus conversion can read.
    #[cfg(feature = "train")]
    Conllu {
        /// The number of the line, from 1.
        line: usize,
        /// What is wrong with the line.
        message: String,
    },
    /// The tokenizer given to the corpus conversion failed.
    #[cfg(feature = "train")]
    Tokenizer(Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    /// Makes a [`Error::Format`].
    ///
    /// # Arguments
    ///
    /// * `line` - The number of the line, from 1.
    /// * `message` - What is wrong with the line.
    ///
    /// # Returns
    ///
    /// The error.
    pub(crate) fn format(line: usize, message: impl Into<String>) -> Self {
        Error::Format {
            line,
            message: message.into(),
        }
    }

    /// Makes a [`Error::Conllu`].
    ///
    /// # Arguments
    ///
    /// * `line` - The number of the line, from 1.
    /// * `message` - What is wrong with the line.
    ///
    /// # Returns
    ///
    /// The error.
    #[cfg(feature = "train")]
    pub(crate) fn conllu(line: usize, message: impl Into<String>) -> Self {
        Error::Conllu {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Format { line, message } => write!(f, "line {line}: {message}"),
            Error::Structure(message) => write!(f, "invalid sentence: {message}"),
            Error::Io(error) => write!(f, "{error}"),
            #[cfg(feature = "train")]
            Error::Conllu { line, message } => write!(f, "line {line}: {message}"),
            #[cfg(feature = "train")]
            Error::Tokenizer(error) => write!(f, "tokenizer: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(error) => Some(error),
            #[cfg(feature = "train")]
            Error::Tokenizer(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}
