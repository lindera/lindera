//! # Lindera PHP Bindings
//!
//! PHP bindings for [Lindera](https://github.com/lindera/lindera), a morphological analysis library for CJK text.
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

pub mod convert;
pub mod dictionary;
pub mod error;
pub mod metadata;
pub mod mode;
pub mod schema;
pub mod segmenter;
pub mod token;
pub mod tokenizer;

#[cfg(feature = "train")]
pub mod trainer;

use ext_php_rs::prelude::*;

/// PHP module entry point.
///
/// Registers all classes with the PHP extension system.
#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    let module = module
        // Token
        .class::<token::PhpToken>()
        // Mode and penalty
        .class::<mode::PhpMode>()
        .class::<mode::PhpPenalty>()
        // Schema types
        .class::<schema::PhpFieldType>()
        .class::<schema::PhpFieldDefinition>()
        .class::<schema::PhpSchema>()
        // Metadata types
        .class::<metadata::PhpCompressionAlgorithm>()
        .class::<metadata::PhpMetadata>()
        // Dictionary types
        .class::<dictionary::PhpDictionary>()
        .class::<dictionary::PhpUserDictionary>()
        // Tokenizer types
        .class::<tokenizer::PhpTokenizerBuilder>()
        .class::<tokenizer::PhpTokenizer>()
        .class::<tokenizer::PhpNbestResult>();

    #[cfg(feature = "train")]
    let module = module.class::<trainer::PhpTrainer>();

    module
}
