//! # Lindera Node.js Bindings
//!
//! Node.js bindings for [Lindera](https://github.com/lindera/lindera), a morphological analysis library for CJK text.
//!
//! Built with [NAPI-RS](https://napi.rs/) for high-performance native Node.js addon support.
//!
//! ## Features
//!
//! - **Dictionary management**: Build, load, and use custom dictionaries
//! - **Tokenization**: Multiple tokenization modes (normal, decompose)
//! - **Filters**: Character and token filtering pipeline
//! - **Training**: Train custom morphological models (with `train` feature)
//! - **User dictionaries**: Support for custom user dictionaries
//! - **TypeScript**: Full type definitions generated automatically

#[macro_use]
extern crate napi_derive;

pub mod dictionary;
pub mod error;
pub mod metadata;
pub mod mode;
pub mod schema;
pub mod token;
pub mod tokenizer;
pub mod util;

#[cfg(feature = "train")]
pub mod trainer;

/// Returns the version of the lindera-nodejs package.
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
