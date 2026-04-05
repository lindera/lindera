//! Token class wrapping the Rust Token data for PHP.
//!
//! This module provides the PhpToken class that exposes morphological analysis
//! results to PHP.

use ext_php_rs::prelude::*;

use lindera::token::Token;

/// Token object wrapping the Rust Token data.
///
/// Provides access to token surface form, byte positions, word ID,
/// and morphological details.
#[php_class]
#[php(name = "Lindera\\Token")]
#[derive(Clone)]
pub struct PhpToken {
    /// Surface form of the token.
    pub surface: String,
    /// Start byte position in the original text.
    pub byte_start: usize,
    /// End byte position in the original text.
    pub byte_end: usize,
    /// Position index of the token.
    pub position: usize,
    /// Word ID in the dictionary.
    pub word_id: u32,
    /// Whether this token is an unknown word (not found in the dictionary).
    pub is_unknown: bool,
    /// Morphological details of the token.
    pub details: Vec<String>,
}

#[php_impl]
impl PhpToken {
    /// Returns the surface form of the token.
    ///
    /// # Returns
    ///
    /// The surface string.
    #[php(getter)]
    pub fn surface(&self) -> String {
        self.surface.clone()
    }

    /// Returns the start byte position in the original text.
    ///
    /// # Returns
    ///
    /// The byte start position.
    #[php(getter)]
    pub fn byte_start(&self) -> i64 {
        self.byte_start as i64
    }

    /// Returns the end byte position in the original text.
    ///
    /// # Returns
    ///
    /// The byte end position.
    #[php(getter)]
    pub fn byte_end(&self) -> i64 {
        self.byte_end as i64
    }

    /// Returns the position index of the token.
    ///
    /// # Returns
    ///
    /// The token position.
    #[php(getter)]
    pub fn position(&self) -> i64 {
        self.position as i64
    }

    /// Returns the word ID in the dictionary.
    ///
    /// # Returns
    ///
    /// The word ID.
    #[php(getter)]
    pub fn word_id(&self) -> i64 {
        self.word_id as i64
    }

    /// Returns whether this token is an unknown word.
    ///
    /// # Returns
    ///
    /// True if the token is unknown.
    #[php(getter)]
    pub fn is_unknown(&self) -> bool {
        self.is_unknown
    }

    /// Returns the morphological details of the token.
    ///
    /// # Returns
    ///
    /// A list of detail strings.
    #[php(getter)]
    pub fn details(&self) -> Vec<String> {
        self.details.clone()
    }

    /// Returns the detail at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the detail to retrieve.
    ///
    /// # Returns
    ///
    /// The detail string if found, otherwise null.
    pub fn get_detail(&self, index: i64) -> Option<String> {
        self.details.get(index as usize).cloned()
    }

    /// Returns a string representation of the token.
    ///
    /// # Returns
    ///
    /// A string describing the token.
    pub fn __to_string(&self) -> String {
        format!(
            "Token(surface='{}', start={}, end={}, position={}, word_id={}, is_unknown={})",
            self.surface,
            self.byte_start,
            self.byte_end,
            self.position,
            self.word_id,
            self.is_unknown
        )
    }
}

impl PhpToken {
    /// Creates a PhpToken from a Lindera Token.
    ///
    /// # Arguments
    ///
    /// * `token` - Lindera Token to convert.
    ///
    /// # Returns
    ///
    /// A new PhpToken instance.
    pub fn from_token(mut token: Token) -> Self {
        let details = token.details().iter().map(|s| s.to_string()).collect();

        Self {
            surface: token.surface.to_string(),
            byte_start: token.byte_start,
            byte_end: token.byte_end,
            position: token.position,
            word_id: token.word_id.id,
            is_unknown: token.word_id.is_unknown(),
            details,
        }
    }
}
