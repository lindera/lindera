//! FFI-independent helpers shared by the Lindera language bindings
//! (`lindera-python`, `lindera-php`, `lindera-ruby`, `lindera-nodejs`,
//! `lindera-wasm`).
//!
//! Each binding wraps the core `lindera` crate for its own FFI layer (PyO3,
//! ext-php-rs, magnus, napi, wasm-bindgen) and historically reimplemented the
//! same pure-Rust conversion logic. This crate holds that shared logic as
//! plain Rust so it can be unit-tested without any FFI toolchain.

pub mod error;
pub mod schema;
pub mod token;

pub use error::{CoreError, CoreResult, ErrorKind};
pub use token::TokenView;
