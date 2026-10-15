#[cfg(feature = "build_rs")]
pub mod assets;
pub mod builder;
pub mod dictionary;
pub mod error;
pub mod loader;
mod macros;
pub mod mode;
pub mod nbest;
/// The source digest `build.rs` computes, compiled here for its unit tests.
#[cfg(all(test, feature = "build_rs"))]
mod source_digest;
pub mod space_penalty;
pub mod util;
pub mod viterbi;
pub mod whitespace;

pub type LinderaResult<T> = Result<T, crate::error::LinderaError>;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERSION
}
