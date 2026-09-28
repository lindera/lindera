#![cfg_attr(docsrs, feature(doc_cfg))]
//! Morphological segmenter of Lindera.
//!
//! This crate provides the [`segmenter::Segmenter`] API and the dictionary
//! loading functions in [`dictionary`]. Up to v6 it was published as the
//! `lindera` crate; `lindera` is now a facade that re-exports this crate, so
//! most applications should depend on `lindera` rather than on this crate.

pub mod dictionary;
pub mod error;
pub mod mode;
pub mod segmenter;
pub mod space_penalty;
pub mod token;
pub mod worker;

/// Result type used throughout Lindera, with [`error::LinderaError`] as the
/// error type.
pub use lindera_dictionary::LinderaResult;

/// Version of this crate, taken from `CARGO_PKG_VERSION` at compile time.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of this crate.
///
/// # Returns
///
/// The crate version string, e.g. `"7.0.0"`.
pub fn get_version() -> &'static str {
    VERSION
}
