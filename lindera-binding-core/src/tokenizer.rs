//! Shared tokenizer build-flow orchestration for the bindings.
//!
//! Each binding's tokenizer wrapper reimplements the same flow: configure a
//! [`lindera_analysis::tokenizer::TokenizerBuilder`], build a
//! [`lindera_analysis::tokenizer::Tokenizer`], and convert the resulting tokens. This
//! module collects that orchestration into [`CoreTokenizerBuilder`] and
//! [`CoreTokenizer`], leaving each binding to do only its FFI-value conversion
//! (`serde_json::Value` ⇔ the host language's argument type) and a thin wrapper.

use std::path::Path;
use std::str::FromStr;

use serde_json::Value;

use lindera::dictionary::{Dictionary, UserDictionary};
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::tokenizer::{Tokenizer, TokenizerBuilder};

use crate::error::CoreResult;
use crate::token::TokenView;

/// Builder that orchestrates tokenizer configuration on behalf of the bindings.
///
/// Wraps [`lindera_analysis::tokenizer::TokenizerBuilder`]; filter arguments are passed
/// as [`serde_json::Value`] so the FFI-specific value conversion stays in each
/// binding.
pub struct CoreTokenizerBuilder {
    /// The backing lindera builder.
    inner: TokenizerBuilder,
}

impl CoreTokenizerBuilder {
    /// Creates a builder with the default (empty) configuration.
    pub fn new() -> CoreResult<Self> {
        Ok(Self {
            inner: TokenizerBuilder::new()?,
        })
    }

    /// Creates a builder from a YAML configuration file.
    pub fn from_file(file_path: &Path) -> CoreResult<Self> {
        Ok(Self {
            inner: TokenizerBuilder::from_file(file_path)?,
        })
    }

    /// Sets the segmenter mode from a string (`"normal"` or `"decompose"`).
    pub fn set_mode(&mut self, mode: &str) -> CoreResult<&mut Self> {
        let mode = Mode::from_str(mode)?;
        self.inner.set_segmenter_mode(&mode);
        Ok(self)
    }

    /// Sets the segmenter dictionary URI / path.
    pub fn set_dictionary(&mut self, uri: &str) -> &mut Self {
        self.inner.set_segmenter_dictionary(uri);
        self
    }

    /// Sets the segmenter user-dictionary URI / path.
    pub fn set_user_dictionary(&mut self, uri: &str) -> &mut Self {
        self.inner.set_segmenter_user_dictionary(uri);
        self
    }

    /// Sets whether whitespace tokens are kept in the output.
    pub fn set_keep_whitespace(&mut self, keep_whitespace: bool) -> &mut Self {
        self.inner.set_segmenter_keep_whitespace(keep_whitespace);
        self
    }

    /// Appends a character filter identified by `kind` with JSON `args`.
    pub fn append_character_filter(&mut self, kind: &str, args: &Value) -> &mut Self {
        self.inner.append_character_filter(kind, args);
        self
    }

    /// Appends a token filter identified by `kind` with JSON `args`.
    pub fn append_token_filter(&mut self, kind: &str, args: &Value) -> &mut Self {
        self.inner.append_token_filter(kind, args);
        self
    }

    /// Builds a [`CoreTokenizer`] from the current configuration.
    pub fn build(&self) -> CoreResult<CoreTokenizer> {
        Ok(CoreTokenizer {
            inner: self.inner.build()?,
        })
    }
}

/// Tokenizer that orchestrates tokenization on behalf of the bindings.
///
/// Wraps [`lindera_analysis::tokenizer::Tokenizer`] and returns owned [`TokenView`]s so
/// the bindings never handle borrowed `lindera` tokens directly.
pub struct CoreTokenizer {
    /// The backing lindera tokenizer.
    inner: Tokenizer,
}

impl CoreTokenizer {
    /// Builds a tokenizer from segmenter parts, parsing `mode` from a string.
    pub fn from_segmenter(
        mode: &str,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
    ) -> CoreResult<Self> {
        let mode = Mode::from_str(mode)?;
        let segmenter = Segmenter::new(mode, dictionary, user_dictionary);
        Ok(Self {
            inner: Tokenizer::new(segmenter),
        })
    }

    /// Wraps an already-built lindera [`Tokenizer`].
    pub fn from_tokenizer(tokenizer: Tokenizer) -> Self {
        Self { inner: tokenizer }
    }

    /// Tokenizes `text`, returning owned [`TokenView`]s.
    pub fn tokenize(&self, text: &str) -> CoreResult<Vec<TokenView>> {
        let tokens = self.inner.tokenize(text)?;
        Ok(tokens.into_iter().map(TokenView::from_token).collect())
    }

    /// Tokenizes `text` and returns the N-best results as `(tokens, cost)` pairs.
    pub fn tokenize_nbest(
        &self,
        text: &str,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> CoreResult<Vec<(Vec<TokenView>, i64)>> {
        let results = self.inner.tokenize_nbest(text, n, unique, cost_threshold)?;
        Ok(results
            .into_iter()
            .map(|(tokens, cost)| {
                (
                    tokens.into_iter().map(TokenView::from_token).collect(),
                    cost,
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_new_succeeds() {
        assert!(CoreTokenizerBuilder::new().is_ok());
    }

    #[test]
    fn set_mode_accepts_known_modes() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        assert!(builder.set_mode("normal").is_ok());
        assert!(builder.set_mode("decompose").is_ok());
    }

    #[test]
    fn set_mode_rejects_unknown_mode() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        assert!(builder.set_mode("definitely-not-a-mode").is_err());
    }

    #[test]
    fn infallible_setters_chain() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        builder
            .set_dictionary("embedded://ipadic")
            .set_keep_whitespace(true)
            .append_token_filter("japanese_compound_word", &Value::Object(Default::default()));
        // Reaching here means the borrow-returning setters compose.
    }
}
