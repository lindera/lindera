//! Token representation for morphological analysis results.
//!
//! This module wraps Lindera tokens for use in Ruby.

use magnus::prelude::*;
use magnus::{Error, Ruby, method};

use lindera::token::Token;

/// Token object wrapping the Rust Token data.
///
/// This class provides access to token fields and details.
#[magnus::wrap(class = "Lindera::Token", free_immediately, size)]
pub struct RbToken {
    /// Surface form of the token.
    surface: String,
    /// Start byte position in the original text.
    byte_start: usize,
    /// End byte position in the original text.
    byte_end: usize,
    /// Position index of the token.
    position: usize,
    /// Word ID in the dictionary.
    word_id: u32,
    /// Whether this token is an unknown word.
    is_unknown: bool,
    /// Morphological details of the token.
    details: Option<Vec<String>>,
}

impl RbToken {
    /// Creates a new `RbToken` from a Lindera `Token`.
    ///
    /// # Arguments
    ///
    /// * `token` - Lindera token to convert.
    ///
    /// # Returns
    ///
    /// A new `RbToken` instance.
    pub fn from_token(mut token: Token) -> Self {
        let details = token.details().iter().map(|s| s.to_string()).collect();

        Self {
            surface: token.surface.to_string(),
            byte_start: token.byte_start,
            byte_end: token.byte_end,
            position: token.position,
            word_id: token.word_id.id,
            is_unknown: token.word_id.is_unknown(),
            details: Some(details),
        }
    }

    /// Returns the surface form of the token.
    fn surface(&self) -> String {
        self.surface.clone()
    }

    /// Returns the start byte position.
    fn byte_start(&self) -> usize {
        self.byte_start
    }

    /// Returns the end byte position.
    fn byte_end(&self) -> usize {
        self.byte_end
    }

    /// Returns the position index.
    fn position(&self) -> usize {
        self.position
    }

    /// Returns the word ID.
    fn word_id(&self) -> u32 {
        self.word_id
    }

    /// Returns whether this token is an unknown word.
    fn is_unknown(&self) -> bool {
        self.is_unknown
    }

    /// Returns the morphological details of the token.
    fn details(&self) -> Option<Vec<String>> {
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
    /// The detail string if found, otherwise nil.
    fn get_detail(&self, index: usize) -> Option<String> {
        self.details.as_ref().and_then(|d| d.get(index).cloned())
    }

    /// Returns the string representation of the token.
    fn to_s(&self) -> String {
        self.surface.clone()
    }

    /// Returns the inspect representation of the token.
    fn inspect(&self) -> String {
        format!(
            "#<Lindera::Token surface='{}', start={}, end={}, position={}, word_id={}, unknown={}>",
            self.surface,
            self.byte_start,
            self.byte_end,
            self.position,
            self.word_id,
            self.is_unknown
        )
    }
}

/// Defines the Token class in the given Ruby module.
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
    let token_class = module.define_class("Token", ruby.class_object())?;
    token_class.define_method("surface", method!(RbToken::surface, 0))?;
    token_class.define_method("byte_start", method!(RbToken::byte_start, 0))?;
    token_class.define_method("byte_end", method!(RbToken::byte_end, 0))?;
    token_class.define_method("position", method!(RbToken::position, 0))?;
    token_class.define_method("word_id", method!(RbToken::word_id, 0))?;
    token_class.define_method("unknown?", method!(RbToken::is_unknown, 0))?;
    token_class.define_method("details", method!(RbToken::details, 0))?;
    token_class.define_method("get_detail", method!(RbToken::get_detail, 1))?;
    token_class.define_method("to_s", method!(RbToken::to_s, 0))?;
    token_class.define_method("inspect", method!(RbToken::inspect, 0))?;

    Ok(())
}
