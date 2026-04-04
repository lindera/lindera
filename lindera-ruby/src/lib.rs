//! # Lindera Ruby Bindings
//!
//! Ruby bindings for [Lindera](https://github.com/lindera/lindera), a morphological analysis library for CJK text.
//!
//! Lindera provides high-performance tokenization and morphological analysis for:
//! - Japanese (IPADIC, IPADIC NEologd, UniDic)
//! - Korean (ko-dic)
//! - Chinese (CC-CEDICT, Jieba)
//!
//! ## Features
//!
//! - **Dictionary management**: Build, load, and use custom dictionaries
//! - **Tokenization**: Multiple tokenization modes (normal, decompose)
//! - **Filters**: Character and token filtering pipeline
//! - **Training**: Train custom morphological models (with `train` feature)
//! - **User dictionaries**: Support for custom user dictionaries
//!
//! ## Examples
//!
//! ```ruby
//! require "lindera"
//!
//! # Create a tokenizer
//! builder = Lindera::TokenizerBuilder.new
//! tokenizer = builder.build
//!
//! # Tokenize text
//! tokens = tokenizer.tokenize("関西国際空港")
//! tokens.each { |token| puts "#{token.surface}: #{token.details}" }
//! ```

pub mod character_filter;
pub mod dictionary;
pub mod error;
pub mod metadata;
pub mod mode;
pub mod schema;
pub mod segmenter;
pub mod token;
pub mod token_filter;
pub mod tokenizer;
pub mod util;

#[cfg(feature = "train")]
pub mod trainer;

use magnus::{Error, Ruby, function};

/// Returns the version of the lindera-ruby package.
///
/// # Returns
///
/// A string with the package version.
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Ruby extension initialization entry point.
///
/// Defines the `Lindera` module and all its classes and functions.
#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Lindera")?;

    // Version
    module.define_module_function("version", function!(version, 0))?;

    // Schema / FieldDefinition / FieldType
    schema::define(ruby, &module)?;

    // Metadata / CompressionAlgorithm
    metadata::define(ruby, &module)?;

    // Mode / Penalty
    mode::define(ruby, &module)?;

    // Token
    token::define(ruby, &module)?;

    // Dictionary / UserDictionary + load/build functions
    dictionary::define(ruby, &module)?;

    // TokenizerBuilder / Tokenizer
    tokenizer::define(ruby, &module)?;

    // Trainer (feature = "train")
    #[cfg(feature = "train")]
    trainer::define(ruby, &module)?;

    Ok(())
}
