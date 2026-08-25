//! Tokenizer implementation for morphological analysis.
//!
//! This module provides a builder pattern for creating tokenizers and the tokenizer itself.
//! The build-flow orchestration is delegated to
//! [`lindera_binding_core::CoreTokenizerBuilder`] / [`lindera_binding_core::CoreTokenizer`];
//! this module only adds the napi wrappers and the JS-value conversion.

use std::path::Path;

use lindera_binding_core::{CoreTokenizer, CoreTokenizerBuilder};

use crate::dictionary::{JsDictionary, JsUserDictionary};
use crate::error::to_napi_error;
use crate::token::{NbestResult, Token};
use crate::util::js_value_to_serde_value;

/// Builder for creating a Tokenizer with custom configuration.
///
/// The builder pattern allows for fluent configuration of tokenizer parameters including
/// dictionaries, modes, and filter pipelines.
#[napi(js_name = "TokenizerBuilder")]
pub struct JsTokenizerBuilder {
    inner: CoreTokenizerBuilder,
}

#[napi]
impl JsTokenizerBuilder {
    /// Creates a new TokenizerBuilder with default configuration.
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        let inner = CoreTokenizerBuilder::new().map_err(to_napi_error)?;
        Ok(Self { inner })
    }

    /// Loads configuration from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the configuration file.
    ///
    /// # Returns
    ///
    /// A new TokenizerBuilder with the loaded configuration.
    #[napi]
    pub fn from_file(&self, file_path: String) -> napi::Result<JsTokenizerBuilder> {
        let inner =
            CoreTokenizerBuilder::from_file(Path::new(&file_path)).map_err(to_napi_error)?;
        Ok(JsTokenizerBuilder { inner })
    }

    /// Sets the tokenization mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - Mode string ("normal" or "decompose").
    ///
    /// # Returns
    ///
    /// The builder itself (`this`), enabling method chaining.
    #[napi]
    pub fn set_mode(&mut self, mode: String) -> napi::Result<&Self> {
        self.inner.set_mode(&mode).map_err(to_napi_error)?;
        Ok(self)
    }

    /// Sets the dictionary path or URI.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the dictionary directory or embedded URI (e.g. "embedded://ipadic").
    ///
    /// # Returns
    ///
    /// The builder itself (`this`), enabling method chaining.
    #[napi]
    pub fn set_dictionary(&mut self, path: String) -> &Self {
        self.inner.set_dictionary(&path);
        self
    }

    /// Sets the user dictionary URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - URI to the user dictionary.
    ///
    /// # Returns
    ///
    /// The builder itself (`this`), enabling method chaining.
    #[napi]
    pub fn set_user_dictionary(&mut self, uri: String) -> &Self {
        self.inner.set_user_dictionary(&uri);
        self
    }

    /// Sets whether to keep whitespace in tokenization results.
    ///
    /// # Arguments
    ///
    /// * `keep_whitespace` - If true, whitespace tokens will be included in results.
    ///
    /// # Returns
    ///
    /// The builder itself (`this`), enabling method chaining.
    #[napi]
    pub fn set_keep_whitespace(&mut self, keep_whitespace: bool) -> &Self {
        self.inner.set_keep_whitespace(keep_whitespace);
        self
    }

    /// Appends a character filter to the preprocessing pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - Type of character filter to add (e.g. "unicode_normalize", "mapping").
    /// * `args` - Optional filter arguments as a JSON-compatible object.
    ///
    /// # Returns
    ///
    /// The builder itself (`this`), enabling method chaining.
    #[napi]
    pub fn append_character_filter(
        &mut self,
        kind: String,
        args: Option<serde_json::Value>,
    ) -> &Self {
        let filter_args = js_value_to_serde_value(args);
        self.inner.append_character_filter(&kind, &filter_args);
        self
    }

    /// Appends a token filter to the postprocessing pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - Type of token filter to add (e.g. "lowercase", "japanese_stop_tags").
    /// * `args` - Optional filter arguments as a JSON-compatible object.
    ///
    /// # Returns
    ///
    /// The builder itself (`this`), enabling method chaining.
    #[napi]
    pub fn append_token_filter(
        &mut self,
        kind: String,
        args: Option<serde_json::Value>,
    ) -> &Self {
        let filter_args = js_value_to_serde_value(args);
        self.inner.append_token_filter(&kind, &filter_args);
        self
    }

    /// Builds the tokenizer with the configured settings.
    ///
    /// # Returns
    ///
    /// A configured Tokenizer instance ready for use.
    #[napi]
    pub fn build(&self) -> napi::Result<JsTokenizer> {
        let inner = self.inner.build().map_err(to_napi_error)?;
        Ok(JsTokenizer { inner })
    }
}

/// Tokenizer for performing morphological analysis.
///
/// The tokenizer processes text and returns tokens with their morphological features.
#[napi(js_name = "Tokenizer")]
pub struct JsTokenizer {
    inner: CoreTokenizer,
}

#[napi]
impl JsTokenizer {
    /// Creates a new tokenizer with the given dictionary and mode.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - Dictionary to use for tokenization.
    /// * `mode` - Tokenization mode ("normal" or "decompose"). Default: "normal".
    /// * `user_dictionary` - Optional user dictionary for custom words.
    #[napi(constructor)]
    pub fn new(
        dictionary: &JsDictionary,
        mode: Option<String>,
        user_dictionary: Option<&JsUserDictionary>,
    ) -> napi::Result<Self> {
        let mode_str = mode.unwrap_or_else(|| "normal".to_string());
        let dict = dictionary.inner.clone();
        let user_dict = user_dictionary.map(|d| d.inner.clone());

        let inner =
            CoreTokenizer::from_segmenter(&mode_str, dict, user_dict).map_err(to_napi_error)?;

        Ok(Self { inner })
    }

    /// Tokenizes the given text.
    ///
    /// Tokens are returned as plain JS objects owned by the JS heap, so
    /// they are reclaimed by ordinary GC and survive `JSON.stringify` /
    /// `structuredClone` / worker transfer without conversion.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to tokenize.
    ///
    /// # Returns
    ///
    /// An array of token objects containing morphological features, in
    /// reading order.
    #[napi]
    pub fn tokenize(&self, text: String) -> napi::Result<Vec<Token>> {
        let views = self.inner.tokenize(&text).map_err(to_napi_error)?;
        Ok(views.into_iter().map(Token::from_view).collect())
    }

    /// Tokenizes the given text and returns only the token surfaces.
    ///
    /// This is the fast path for wakati-style use: no Token objects are
    /// created and no morphological details are loaded, so it is
    /// significantly faster than `tokenize` when only the surface strings
    /// are needed. The surfaces equal `tokenize(text).map((t) => t.surface)`.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to tokenize.
    ///
    /// # Returns
    ///
    /// An array of surface strings, in reading order.
    #[napi]
    pub fn tokenize_surfaces(&self, text: String) -> napi::Result<Vec<String>> {
        let views = self.inner.tokenize_surfaces(&text).map_err(to_napi_error)?;
        Ok(views.into_iter().map(|view| view.surface).collect())
    }

    /// Tokenizes the given text and returns N-best results.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to tokenize.
    /// * `n` - Number of N-best results to return.
    /// * `unique` - If true, deduplicate results (default: false).
    /// * `cost_threshold` - Maximum cost difference from the best path (default: undefined).
    ///
    /// # Returns
    ///
    /// An array of NbestResult objects, each containing tokens and their cost.
    #[napi]
    pub fn tokenize_nbest(
        &self,
        text: String,
        n: u32,
        unique: Option<bool>,
        cost_threshold: Option<i64>,
    ) -> napi::Result<Vec<NbestResult>> {
        let results = self
            .inner
            .tokenize_nbest(&text, n as usize, unique.unwrap_or(false), cost_threshold)
            .map_err(to_napi_error)?;

        let js_results: Vec<NbestResult> = results
            .into_iter()
            .map(|(views, cost)| NbestResult {
                tokens: views.into_iter().map(Token::from_view).collect(),
                cost,
            })
            .collect();

        Ok(js_results)
    }
}
