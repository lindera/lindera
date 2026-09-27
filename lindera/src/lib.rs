#![cfg_attr(docsrs, feature(doc_cfg))]
//! Lindera is a morphological analysis library for Japanese, Chinese and
//! Korean.
//!
//! This crate is a facade over two crates: `lindera-segmenter`, the
//! morphological segmenter and the dictionary loading API, is re-exported at
//! the crate root ([`dictionary`], [`segmenter`], [`token`], ...), and
//! `lindera-analysis`, the text analysis chain of character filters, token
//! filters and the tokenizer that composes them with a segmenter, is
//! re-exported as `lindera::analysis` when the `analysis` feature is enabled
//! (it is by default). One dependency line gives access to the whole API:
//!
//! ```toml
//! [dependencies]
//! lindera = "7"
//! ```
//!
//! Applications that only need the segmenter can turn the default features
//! off. Doing so also disables `mmap`, the memory-mapped dictionary loading,
//! so enable it again if it is wanted:
//!
//! ```toml
//! [dependencies]
//! lindera = { version = "7", default-features = false, features = ["mmap"] }
//! ```
//!
//! # Example
//!
//! Load a dictionary, wrap it in a [`Segmenter`](segmenter::Segmenter) and
//! run a text through a `Tokenizer`:
//!
//! ```no_run
//! use lindera::dictionary::load_dictionary;
//! use lindera::mode::Mode;
//! use lindera::segmenter::Segmenter;
//!
//! # fn main() -> lindera::LinderaResult<()> {
//! let dictionary = load_dictionary("embedded://ipadic")?;
//! let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
//! # #[cfg(feature = "analysis")]
//! # {
//! use lindera::analysis::tokenizer::Tokenizer;
//!
//! let tokenizer = Tokenizer::new(segmenter);
//! let mut tokens = tokenizer.tokenize("関西国際空港限定トートバッグ")?;
//! for token in tokens.iter_mut() {
//!     let details = token.details().join(",");
//!     println!("{}\t{}", token.surface, details);
//! }
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! # Low-level API
//!
//! The building blocks of `lindera-dictionary` (the dictionary data
//! structures, the dictionary builder, the Viterbi lattice, ...) are reachable
//! under [`dictionary::core`], [`dictionary::builder`], [`dictionary::viterbi`]
//! and the other modules re-exported from [`dictionary`].

#[doc(inline)]
pub use lindera_segmenter::{
    LinderaResult, dictionary, error, mode, segmenter, space_penalty, token, worker,
};

/// The text analysis chain of `lindera-analysis`: character filters, token
/// filters and the `Tokenizer` that composes them with a
/// [`Segmenter`](segmenter::Segmenter).
///
/// Available with the `analysis` feature, which is enabled by default.
#[cfg(feature = "analysis")]
#[cfg_attr(docsrs, doc(cfg(feature = "analysis")))]
#[doc(inline)]
pub use lindera_analysis as analysis;

/// Version of this crate, taken from `CARGO_PKG_VERSION` at compile time.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of this crate.
///
/// # Returns
///
/// The crate version string, e.g. `"6.2.0"`.
pub fn get_version() -> &'static str {
    VERSION
}
