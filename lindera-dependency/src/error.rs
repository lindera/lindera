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
    /// The chunker was given something it cannot use: a template or a list
    /// of columns that cannot be read, weights that are not finite, or
    /// training data that cannot be trained on.
    Chunker(String),
    /// Bytes are not a model file this crate reads: another magic number or
    /// version, bytes that end early or go on after the end, or content
    /// that is not what the format says.
    Model(String),
    /// A model was used with another dictionary than the one it was trained
    /// for: the name of the dictionary differs, or a morpheme has fewer
    /// feature columns than the schema of the model reads.
    Dictionary(String),
    /// No model is embedded in the build for the dictionary asked for: no
    /// feature embeds one for it, the build is without the feature, or it
    /// is a documentation build, which embeds an empty file.
    Embedded(String),
    /// The parser was given something it cannot use: a feature set that
    /// cannot be read, or weights that are not finite.
    Parser(String),
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
    /// The files given to the evaluation cannot be compared: they do not
    /// hold the same number of sentences.
    #[cfg(feature = "train")]
    Evaluation(String),
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
            Error::Chunker(message) => write!(f, "{message}"),
            Error::Model(message) => write!(f, "{message}"),
            Error::Dictionary(message) => write!(f, "{message}"),
            Error::Embedded(message) => write!(f, "{message}"),
            Error::Parser(message) => write!(f, "{message}"),
            #[cfg(feature = "train")]
            Error::Conllu { line, message } => write!(f, "line {line}: {message}"),
            #[cfg(feature = "train")]
            Error::Tokenizer(error) => write!(f, "tokenizer: {error}"),
            #[cfg(feature = "train")]
            Error::Evaluation(message) => write!(f, "{message}"),
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
