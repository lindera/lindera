//! Token representation for morphological analysis results.
//!
//! This module provides the Token class that wraps morphological analysis results
//! and exposes token properties to JavaScript.

use lindera::token::Token;

/// A morphological token.
///
/// Represents a single token from morphological analysis with its surface form,
/// position information, and morphological details.
#[napi(js_name = "Token")]
#[derive(Clone)]
pub struct JsToken {
    /// Surface form of the token.
    surface: String,
    /// Start byte position in the original text.
    byte_start: u32,
    /// End byte position in the original text.
    byte_end: u32,
    /// Position index of the token.
    position: u32,
    /// Word ID in the dictionary.
    word_id: u32,
    /// Whether this token is an unknown word.
    is_unknown: bool,
    /// Morphological details of the token.
    details: Option<Vec<String>>,
}

#[napi]
impl JsToken {
    /// Surface form of the token.
    #[napi(getter)]
    pub fn surface(&self) -> String {
        self.surface.clone()
    }

    /// Start byte position in the original text.
    #[napi(getter)]
    pub fn byte_start(&self) -> u32 {
        self.byte_start
    }

    /// End byte position in the original text.
    #[napi(getter)]
    pub fn byte_end(&self) -> u32 {
        self.byte_end
    }

    /// Position index of the token.
    #[napi(getter)]
    pub fn position(&self) -> u32 {
        self.position
    }

    /// Word ID in the dictionary.
    #[napi(getter)]
    pub fn word_id(&self) -> u32 {
        self.word_id
    }

    /// Whether this token is an unknown word (not found in the dictionary).
    #[napi(getter)]
    pub fn is_unknown(&self) -> bool {
        self.is_unknown
    }

    /// Morphological details of the token (part of speech, reading, etc.).
    #[napi(getter)]
    pub fn details(&self) -> Option<Vec<String>> {
        self.details.clone()
    }

    /// Returns the detail string at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index into the details array.
    ///
    /// # Returns
    ///
    /// The detail string if found, or `null` if the index is out of range.
    #[napi]
    pub fn get_detail(&self, index: u32) -> Option<String> {
        self.details
            .as_ref()
            .and_then(|d| d.get(index as usize).cloned())
    }
}

impl JsToken {
    /// Creates a JsToken from a lindera Token.
    ///
    /// # Arguments
    ///
    /// * `token` - The lindera Token to convert.
    ///
    /// # Returns
    ///
    /// A new JsToken instance.
    pub fn from_token(token: Token) -> Self {
        Self::from_view(lindera_binding_core::TokenView::from_token(token))
    }

    /// Creates a JsToken from a binding-core `TokenView`.
    ///
    /// # Arguments
    ///
    /// * `view` - The token view produced by the binding-core tokenizer.
    ///
    /// # Returns
    ///
    /// A new JsToken instance.
    pub fn from_view(view: lindera_binding_core::TokenView) -> Self {
        Self {
            surface: view.surface,
            byte_start: view.byte_start as u32,
            byte_end: view.byte_end as u32,
            position: view.position as u32,
            word_id: view.word_id,
            is_unknown: view.is_unknown,
            details: Some(view.details),
        }
    }
}

/// N-best tokenization result.
///
/// Contains a list of tokens and their total path cost.
#[napi(js_name = "NbestResult")]
pub struct JsNbestResult {
    /// Tokens in this result.
    tokens: Vec<JsToken>,
    /// Total path cost of this tokenization.
    cost: i64,
}

#[napi]
impl JsNbestResult {
    /// Tokens in this result.
    #[napi(getter)]
    pub fn tokens(&self) -> Vec<JsToken> {
        self.tokens.clone()
    }

    /// Total path cost of this tokenization.
    #[napi(getter)]
    pub fn cost(&self) -> i64 {
        self.cost
    }
}

impl JsNbestResult {
    /// Creates a new JsNbestResult.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The tokens in this result.
    /// * `cost` - The total path cost.
    ///
    /// # Returns
    ///
    /// A new JsNbestResult instance.
    pub fn new(tokens: Vec<JsToken>, cost: i64) -> Self {
        Self { tokens, cost }
    }
}
