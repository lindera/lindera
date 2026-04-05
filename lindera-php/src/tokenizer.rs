//! Tokenizer implementation for morphological analysis in PHP.
//!
//! This module provides a builder pattern for creating tokenizers and the tokenizer itself.

use std::cell::RefCell;
use std::path::Path;
use std::str::FromStr;

use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::{Tokenizer, TokenizerBuilder};

use crate::convert::zval_to_value;
use crate::dictionary::{PhpDictionary, PhpUserDictionary};
use crate::error::lindera_value_err;
use crate::token::PhpToken;

/// Builder for creating a Tokenizer with custom configuration.
///
/// The builder allows configuration of dictionaries, modes, and filter pipelines.
/// Methods mutate internal state; call build() to create the Tokenizer.
#[php_class]
#[php(name = "Lindera\\TokenizerBuilder")]
pub struct PhpTokenizerBuilder {
    /// The inner builder (wrapped in RefCell for interior mutability).
    inner: RefCell<TokenizerBuilder>,
}

#[php_impl]
impl PhpTokenizerBuilder {
    /// Creates a new TokenizerBuilder with default configuration.
    ///
    /// # Returns
    ///
    /// A new TokenizerBuilder instance.
    pub fn __construct() -> PhpResult<Self> {
        let inner = TokenizerBuilder::new().map_err(|err| {
            lindera_value_err(format!("Failed to create TokenizerBuilder: {err}"))
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
    pub fn from_file(&self, file_path: String) -> PhpResult<()> {
        let builder = TokenizerBuilder::from_file(Path::new(&file_path))
            .map_err(|err| lindera_value_err(format!("Failed to load config from file: {err}")))?;

        *self.inner.borrow_mut() = builder;
        Ok(())
    }

    /// Sets the tokenization mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - Mode string ("normal" or "decompose").
    pub fn set_mode(&self, mode: String) -> PhpResult<()> {
        let m = Mode::from_str(&mode)
            .map_err(|err| lindera_value_err(format!("Failed to create mode: {err}")))?;

        self.inner.borrow_mut().set_segmenter_mode(&m);
        Ok(())
    }

    /// Sets the dictionary path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the dictionary directory or embedded dictionary name.
    pub fn set_dictionary(&self, path: String) -> PhpResult<()> {
        self.inner.borrow_mut().set_segmenter_dictionary(&path);
        Ok(())
    }

    /// Sets the user dictionary URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - URI to the user dictionary.
    pub fn set_user_dictionary(&self, uri: String) -> PhpResult<()> {
        self.inner.borrow_mut().set_segmenter_user_dictionary(&uri);
        Ok(())
    }

    /// Sets whether to keep whitespace in tokenization results.
    ///
    /// # Arguments
    ///
    /// * `keep_whitespace` - If true, whitespace tokens will be included.
    pub fn set_keep_whitespace(&self, keep_whitespace: bool) -> PhpResult<()> {
        self.inner
            .borrow_mut()
            .set_segmenter_keep_whitespace(keep_whitespace);
        Ok(())
    }

    /// Appends a character filter to the filter pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - Type of character filter to add.
    /// * `args` - Optional Zval containing filter arguments (associative array).
    pub fn append_character_filter(&self, kind: String, args: Option<&Zval>) -> PhpResult<()> {
        let filter_args = if let Some(zval) = args {
            zval_to_value(zval)?
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
    /// * `args` - Optional Zval containing filter arguments (associative array).
    pub fn append_token_filter(&self, kind: String, args: Option<&Zval>) -> PhpResult<()> {
        let filter_args = if let Some(zval) = args {
            zval_to_value(zval)?
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
    /// A configured Tokenizer instance ready for use.
    pub fn build(&self) -> PhpResult<PhpTokenizer> {
        let tokenizer = self
            .inner
            .borrow()
            .build()
            .map_err(|err| lindera_value_err(format!("Failed to build tokenizer: {err}")))?;

        Ok(PhpTokenizer { inner: tokenizer })
    }
}

/// Tokenizer for performing morphological analysis.
///
/// The tokenizer processes text and returns tokens with their morphological features.
#[php_class]
#[php(name = "Lindera\\Tokenizer")]
pub struct PhpTokenizer {
    /// The inner Lindera tokenizer.
    inner: Tokenizer,
}

#[php_impl]
impl PhpTokenizer {
    /// Creates a new tokenizer with the given dictionary and mode.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - Dictionary to use for tokenization.
    /// * `mode` - Tokenization mode ("normal" or "decompose"). Default: "normal".
    /// * `user_dictionary` - Optional user dictionary for custom words.
    ///
    /// # Returns
    ///
    /// A new Tokenizer instance.
    pub fn __construct(
        dictionary: &PhpDictionary,
        mode: Option<String>,
        user_dictionary: Option<&PhpUserDictionary>,
    ) -> PhpResult<Self> {
        let mode_str = mode.unwrap_or_else(|| "normal".to_string());
        let m = Mode::from_str(&mode_str)
            .map_err(|err| lindera_value_err(format!("Failed to create mode: {err}")))?;

        let dict = dictionary.inner.clone();
        let user_dict = user_dictionary.map(|d| d.inner.clone());

        let segmenter = Segmenter::new(m, dict, user_dict);
        let tokenizer = Tokenizer::new(segmenter);

        Ok(Self { inner: tokenizer })
    }

    /// Tokenizes the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to tokenize.
    ///
    /// # Returns
    ///
    /// A list of Token objects containing morphological features.
    pub fn tokenize(&self, text: String) -> PhpResult<Vec<PhpToken>> {
        let tokens = self
            .inner
            .tokenize(&text)
            .map_err(|err| lindera_value_err(format!("Failed to tokenize text: {err}")))?;

        let php_tokens: Vec<PhpToken> = tokens.into_iter().map(PhpToken::from_token).collect();

        Ok(php_tokens)
    }

    /// Tokenizes the given text and returns N-best results.
    ///
    /// Returns an array of associative arrays, each containing:
    /// - "tokens": array of Token objects
    /// - "cost": integer cost value
    ///
    /// # Arguments
    ///
    /// * `text` - Text to tokenize.
    /// * `n` - Number of N-best results to return.
    /// * `unique` - If true, only return unique segmentations. Default: false.
    /// * `cost_threshold` - Optional cost threshold for filtering results.
    ///
    /// # Returns
    ///
    /// A Zval containing an array of {"tokens": [...], "cost": int} entries.
    pub fn tokenize_nbest(
        &self,
        text: String,
        n: i64,
        unique: Option<bool>,
        cost_threshold: Option<i64>,
    ) -> PhpResult<Vec<PhpNbestResult>> {
        let unique_flag = unique.unwrap_or(false);
        let results = self
            .inner
            .tokenize_nbest(&text, n as usize, unique_flag, cost_threshold)
            .map_err(|err| lindera_value_err(format!("Failed to tokenize_nbest text: {err}")))?;

        let php_results: Vec<PhpNbestResult> = results
            .into_iter()
            .map(|(tokens, cost)| {
                let php_tokens = tokens.into_iter().map(PhpToken::from_token).collect();
                PhpNbestResult {
                    tokens: php_tokens,
                    cost,
                }
            })
            .collect();

        Ok(php_results)
    }
}

/// N-best tokenization result containing tokens and their cost.
#[php_class]
#[php(name = "Lindera\\NbestResult")]
pub struct PhpNbestResult {
    /// The tokens in this result.
    tokens: Vec<PhpToken>,
    /// The total cost of this segmentation.
    cost: i64,
}

#[php_impl]
impl PhpNbestResult {
    /// Returns the tokens for this N-best result.
    ///
    /// # Returns
    ///
    /// A list of Token objects.
    #[php(getter)]
    pub fn tokens(&self) -> Vec<PhpToken> {
        self.tokens.clone()
    }

    /// Returns the cost of this segmentation.
    ///
    /// # Returns
    ///
    /// The cost value.
    #[php(getter)]
    pub fn cost(&self) -> i64 {
        self.cost
    }

    /// Returns a string representation.
    ///
    /// # Returns
    ///
    /// A string describing the N-best result.
    pub fn __to_string(&self) -> String {
        format!(
            "NbestResult(tokens={}, cost={})",
            self.tokens.len(),
            self.cost
        )
    }
}
