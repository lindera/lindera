//! Tokenizer implementation for morphological analysis.
//!
//! This module provides a builder pattern for creating tokenizers and the tokenizer itself.

use std::cell::RefCell;
use std::path::Path;
use std::str::FromStr;

use magnus::prelude::*;
use magnus::{Error, RArray, RHash, Ruby, function, method};

use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::{Tokenizer, TokenizerBuilder};

use crate::dictionary::{RbDictionary, RbUserDictionary};
use crate::error::to_magnus_error;
use crate::token::RbToken;
use crate::util::rb_hash_to_json;

/// Builder for creating a `Tokenizer` with custom configuration.
///
/// The builder pattern allows for fluent configuration of tokenizer parameters.
/// Uses `RefCell` for interior mutability since Magnus `method!` requires `&self`.
#[magnus::wrap(class = "Lindera::TokenizerBuilder", free_immediately, size)]
pub struct RbTokenizerBuilder {
    /// Inner Lindera tokenizer builder (wrapped in RefCell for interior mutability).
    inner: RefCell<TokenizerBuilder>,
}

impl RbTokenizerBuilder {
    /// Creates a new `RbTokenizerBuilder` with default configuration.
    ///
    /// # Returns
    ///
    /// A new `RbTokenizerBuilder` instance.
    fn new() -> Result<Self, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let inner = TokenizerBuilder::new().map_err(|err| {
            to_magnus_error(&ruby, format!("Failed to create TokenizerBuilder: {err}"))
        })?;
        Ok(Self {
            inner: RefCell::new(inner),
        })
    }

    /// Loads configuration from a file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the configuration file.
    ///
    /// # Returns
    ///
    /// A new `RbTokenizerBuilder` with the loaded configuration.
    fn from_file(file_path: String) -> Result<Self, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let inner = TokenizerBuilder::from_file(Path::new(&file_path)).map_err(|err| {
            to_magnus_error(&ruby, format!("Failed to load config from file: {err}"))
        })?;
        Ok(Self {
            inner: RefCell::new(inner),
        })
    }

    /// Sets the tokenization mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - Mode string ("normal" or "decompose").
    fn set_mode(&self, mode: String) -> Result<(), Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let m = Mode::from_str(&mode)
            .map_err(|err| to_magnus_error(&ruby, format!("Failed to create mode: {err}")))?;
        self.inner.borrow_mut().set_segmenter_mode(&m);
        Ok(())
    }

    /// Sets the dictionary path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the dictionary directory.
    fn set_dictionary(&self, path: String) {
        self.inner.borrow_mut().set_segmenter_dictionary(&path);
    }

    /// Sets the user dictionary URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - URI to the user dictionary.
    fn set_user_dictionary(&self, uri: String) {
        self.inner.borrow_mut().set_segmenter_user_dictionary(&uri);
    }

    /// Sets whether to keep whitespace in tokenization results.
    ///
    /// # Arguments
    ///
    /// * `keep_whitespace` - If true, whitespace tokens will be included.
    fn set_keep_whitespace(&self, keep_whitespace: bool) {
        self.inner
            .borrow_mut()
            .set_segmenter_keep_whitespace(keep_whitespace);
    }

    /// Appends a character filter to the filter pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - Type of character filter to add.
    /// * `args` - Optional hash of filter arguments.
    fn append_character_filter(&self, kind: String, args: Option<RHash>) -> Result<(), Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let filter_args = if let Some(hash) = args {
            rb_hash_to_json(&ruby, hash)?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };
        self.inner
            .borrow_mut()
            .append_character_filter(&kind, &filter_args);
        Ok(())
    }

    /// Appends a token filter to the filter pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - Type of token filter to add.
    /// * `args` - Optional hash of filter arguments.
    fn append_token_filter(&self, kind: String, args: Option<RHash>) -> Result<(), Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let filter_args = if let Some(hash) = args {
            rb_hash_to_json(&ruby, hash)?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };
        self.inner
            .borrow_mut()
            .append_token_filter(&kind, &filter_args);
        Ok(())
    }

    /// Builds the tokenizer with the configured settings.
    ///
    /// # Returns
    ///
    /// A configured `RbTokenizer` instance.
    fn build(&self) -> Result<RbTokenizer, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let tokenizer =
            self.inner.borrow().build().map_err(|err| {
                to_magnus_error(&ruby, format!("Failed to build tokenizer: {err}"))
            })?;
        Ok(RbTokenizer { inner: tokenizer })
    }
}

/// Tokenizer for performing morphological analysis.
///
/// The tokenizer processes text and returns tokens with their morphological features.
#[magnus::wrap(class = "Lindera::Tokenizer", free_immediately, size)]
pub struct RbTokenizer {
    /// Inner Lindera tokenizer.
    inner: Tokenizer,
}

/// Creates a new tokenizer with the given dictionary and mode.
///
/// # Arguments
///
/// * `dictionary` - Dictionary to use.
/// * `mode` - Tokenization mode ("normal" or "decompose"). Defaults to "normal".
/// * `user_dictionary` - Optional user dictionary.
///
/// # Returns
///
/// A new `RbTokenizer` instance.
fn tokenizer_new(
    dictionary: &RbDictionary,
    mode: Option<String>,
    user_dictionary: Option<&RbUserDictionary>,
) -> Result<RbTokenizer, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not initialized");
    let mode_str = mode.as_deref().unwrap_or("normal");
    let m = Mode::from_str(mode_str)
        .map_err(|err| to_magnus_error(&ruby, format!("Failed to create mode: {err}")))?;

    let dict = dictionary.inner.clone();
    let user_dict = user_dictionary.map(|d| d.inner.clone());

    let segmenter = Segmenter::new(m, dict, user_dict);
    let tokenizer = Tokenizer::new(segmenter);

    Ok(RbTokenizer { inner: tokenizer })
}

impl RbTokenizer {
    /// Tokenizes the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to tokenize.
    ///
    /// # Returns
    ///
    /// An array of Token objects.
    fn tokenize(&self, text: String) -> Result<RArray, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let tokens = self
            .inner
            .tokenize(&text)
            .map_err(|err| to_magnus_error(&ruby, format!("Failed to tokenize text: {err}")))?;

        let rb_tokens: Vec<RbToken> = tokens.into_iter().map(RbToken::from_token).collect();
        let arr = ruby.ary_new_capa(rb_tokens.len());
        for token in rb_tokens {
            arr.push(ruby.into_value(token))?;
        }
        Ok(arr)
    }

    /// Tokenizes the given text and returns N-best results.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to tokenize.
    /// * `n` - Number of N-best results.
    /// * `unique` - Whether to deduplicate results (default: false).
    /// * `cost_threshold` - Optional cost threshold.
    ///
    /// # Returns
    ///
    /// An array of [tokens, cost] pairs.
    fn tokenize_nbest(
        &self,
        text: String,
        n: usize,
        unique: Option<bool>,
        cost_threshold: Option<i64>,
    ) -> Result<RArray, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let results = self
            .inner
            .tokenize_nbest(&text, n, unique.unwrap_or(false), cost_threshold)
            .map_err(|err| {
                to_magnus_error(&ruby, format!("Failed to tokenize_nbest text: {err}"))
            })?;

        let rb_results = ruby.ary_new_capa(results.len());
        for (tokens, cost) in results {
            let rb_tokens: Vec<RbToken> = tokens.into_iter().map(RbToken::from_token).collect();
            let token_arr = ruby.ary_new_capa(rb_tokens.len());
            for token in rb_tokens {
                token_arr.push(ruby.into_value(token))?;
            }
            let pair = ruby.ary_new_capa(2);
            pair.push(token_arr)?;
            pair.push(cost)?;
            rb_results.push(pair)?;
        }

        Ok(rb_results)
    }
}

/// Defines TokenizerBuilder and Tokenizer classes in the given Ruby module.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `module` - Parent Ruby module.
///
/// # Returns
///
/// `Ok(())` on success, or a Magnus `Error` on failure.
pub fn define(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    let builder_class = module.define_class("TokenizerBuilder", ruby.class_object())?;
    builder_class.define_singleton_method("new", function!(RbTokenizerBuilder::new, 0))?;
    builder_class
        .define_singleton_method("from_file", function!(RbTokenizerBuilder::from_file, 1))?;
    builder_class.define_method("set_mode", method!(RbTokenizerBuilder::set_mode, 1))?;
    builder_class.define_method(
        "set_dictionary",
        method!(RbTokenizerBuilder::set_dictionary, 1),
    )?;
    builder_class.define_method(
        "set_user_dictionary",
        method!(RbTokenizerBuilder::set_user_dictionary, 1),
    )?;
    builder_class.define_method(
        "set_keep_whitespace",
        method!(RbTokenizerBuilder::set_keep_whitespace, 1),
    )?;
    builder_class.define_method(
        "append_character_filter",
        method!(RbTokenizerBuilder::append_character_filter, 2),
    )?;
    builder_class.define_method(
        "append_token_filter",
        method!(RbTokenizerBuilder::append_token_filter, 2),
    )?;
    builder_class.define_method("build", method!(RbTokenizerBuilder::build, 0))?;

    let tokenizer_class = module.define_class("Tokenizer", ruby.class_object())?;
    tokenizer_class.define_singleton_method("new", function!(tokenizer_new, 3))?;
    tokenizer_class.define_method("tokenize", method!(RbTokenizer::tokenize, 1))?;
    tokenizer_class.define_method("tokenize_nbest", method!(RbTokenizer::tokenize_nbest, 4))?;

    Ok(())
}
