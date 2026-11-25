//! Token representation for morphological analysis results.
//!
//! This module wraps Lindera tokens for use in Ruby.

use magnus::prelude::*;
use magnus::{Error, Integer, RHash, Ruby, method};

use lindera::token::Token;

use crate::util::integer_arg;

/// Token object wrapping the Rust Token data.
///
/// This class provides access to token fields and details.
/// The bunsetsu of a token and what that bunsetsu depends on: what the
/// token filter `japanese_dependency` writes on a token.
///
/// The bunsetsu of a sentence are numbered from 0, and each of them but the
/// last depends on a later one, its head.
#[derive(Clone)]
#[magnus::wrap(class = "Lindera::TokenDependency", free_immediately, size)]
pub struct RbTokenDependency {
    /// The number of the sentence in the text, from 0.
    sentence: u32,
    /// The number of the bunsetsu of the token in its sentence, from 0.
    bunsetsu: u32,
    /// The number of the bunsetsu the bunsetsu of the token depends on.
    head: Option<u32>,
    /// What the token is in its bunsetsu.
    role: &'static str,
    /// The score of the decision that attached the bunsetsu to its head.
    score: f64,
}

impl RbTokenDependency {
    /// Creates an `RbTokenDependency` from a lindera-binding
    /// `DependencyView`.
    ///
    /// # Arguments
    ///
    /// * `view` - The dependency of a token view.
    ///
    /// # Returns
    ///
    /// A new `RbTokenDependency` instance.
    fn from_view(view: lindera_binding::DependencyView) -> Self {
        Self {
            sentence: view.sentence,
            bunsetsu: view.bunsetsu,
            head: view.head,
            role: view.role(),
            score: view.score,
        }
    }

    /// Returns the number of the sentence in the text, from 0.
    fn sentence(&self) -> u32 {
        self.sentence
    }

    /// Returns the number of the bunsetsu of the token in its sentence,
    /// from 0.
    fn bunsetsu(&self) -> u32 {
        self.bunsetsu
    }

    /// Returns the number of the bunsetsu that the bunsetsu of the token
    /// depends on, or nil for the last bunsetsu of a sentence.
    fn head(&self) -> Option<u32> {
        self.head
    }

    /// Returns what the token is in its bunsetsu: `head`, `function`,
    /// `head_function` or `other`.
    fn role(&self) -> &'static str {
        self.role
    }

    /// Returns the score of the decision that attached the bunsetsu to its
    /// head; 0 where there was nothing to decide.
    fn score(&self) -> f64 {
        self.score
    }

    /// Returns the dependency as a plain Hash.
    ///
    /// # Returns
    ///
    /// A Hash with the symbol keys `:sentence`, `:bunsetsu`, `:head` (nil
    /// for the last bunsetsu of a sentence), `:role` and `:score`.
    fn to_hash(ruby: &Ruby, rb_self: &Self) -> Result<RHash, Error> {
        let hash = ruby.hash_new();
        hash.aset(ruby.to_symbol("sentence"), rb_self.sentence)?;
        hash.aset(ruby.to_symbol("bunsetsu"), rb_self.bunsetsu)?;
        hash.aset(ruby.to_symbol("head"), rb_self.head)?;
        hash.aset(ruby.to_symbol("role"), rb_self.role)?;
        hash.aset(ruby.to_symbol("score"), rb_self.score)?;
        Ok(hash)
    }

    /// Returns the inspect representation of the dependency.
    fn inspect(&self) -> String {
        let head = match self.head {
            Some(head) => head.to_string(),
            None => "nil".to_string(),
        };
        format!(
            "#<Lindera::TokenDependency sentence={}, bunsetsu={}, head={}, role='{}', score={}>",
            self.sentence, self.bunsetsu, head, self.role, self.score
        )
    }
}

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
    details: Vec<String>,
    /// The bunsetsu of the token and its head, when the token filter
    /// `japanese_dependency` ran.
    dependency: Option<RbTokenDependency>,
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
    pub fn from_token(token: Token) -> Self {
        Self::from_view(lindera_binding::TokenView::from_token(token))
    }

    /// Creates a new `RbToken` from a lindera-binding `TokenView`.
    ///
    /// # Arguments
    ///
    /// * `view` - Token view produced by the lindera-binding tokenizer.
    ///
    /// # Returns
    ///
    /// A new `RbToken` instance.
    pub fn from_view(view: lindera_binding::TokenView) -> Self {
        Self {
            surface: view.surface,
            byte_start: view.byte_start,
            byte_end: view.byte_end,
            position: view.position,
            word_id: view.word_id,
            is_unknown: view.is_unknown,
            details: view.details,
            dependency: view.dependency.map(RbTokenDependency::from_view),
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
    fn details(&self) -> Vec<String> {
        self.details.clone()
    }

    /// Returns the bunsetsu of the token and its head, or nil unless the
    /// token filter `japanese_dependency` ran.
    fn dependency(&self) -> Option<RbTokenDependency> {
        self.dependency.clone()
    }

    /// Returns the detail at the specified index.
    ///
    /// # Arguments
    ///
    /// * `ruby` - Ruby runtime handle (passed by magnus, not by the caller).
    /// * `rb_self` - The token.
    /// * `index` - Index of the detail to retrieve, 0 or more.
    ///
    /// # Returns
    ///
    /// The detail string if found, nil if the index is past the last detail,
    /// or a `RangeError` for an index that is negative or larger than a
    /// `usize` holds.
    fn get_detail(ruby: &Ruby, rb_self: &Self, index: Integer) -> Result<Option<String>, Error> {
        let index: usize = integer_arg(ruby, "index", index)?;
        Ok(rb_self.details.get(index).cloned())
    }

    /// Returns the token as a plain Hash keyed by Symbols.
    ///
    /// Values keep their natural Ruby types (Integer for the positions and
    /// word id, true/false for `:is_unknown`, Array of String for
    /// `:details`), so the result works directly with `JSON.generate`.
    ///
    /// # Returns
    ///
    /// A Hash with the keys `:surface`, `:byte_start`, `:byte_end`,
    /// `:position`, `:word_id`, `:is_unknown`, `:details`, and
    /// `:dependency` (nil, or the Hash of `TokenDependency#to_h`), or a
    /// Magnus `Error` if the Hash cannot be built.
    fn to_hash(ruby: &Ruby, rb_self: &Self) -> Result<RHash, Error> {
        let this = rb_self;
        let hash = ruby.hash_new();
        hash.aset(ruby.to_symbol("surface"), this.surface.as_str())?;
        hash.aset(ruby.to_symbol("byte_start"), this.byte_start)?;
        hash.aset(ruby.to_symbol("byte_end"), this.byte_end)?;
        hash.aset(ruby.to_symbol("position"), this.position)?;
        hash.aset(ruby.to_symbol("word_id"), this.word_id)?;
        hash.aset(ruby.to_symbol("is_unknown"), this.is_unknown)?;
        let details = ruby.ary_new();
        for detail in &this.details {
            details.push(detail.as_str())?;
        }
        hash.aset(ruby.to_symbol("details"), details)?;
        match &this.dependency {
            Some(dependency) => hash.aset(
                ruby.to_symbol("dependency"),
                RbTokenDependency::to_hash(ruby, dependency)?,
            )?,
            None => hash.aset(ruby.to_symbol("dependency"), ruby.qnil())?,
        }
        Ok(hash)
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
    token_class.define_method("to_hash", method!(RbToken::to_hash, 0))?;
    token_class.define_method("to_h", method!(RbToken::to_hash, 0))?;

    token_class.define_method("dependency", method!(RbToken::dependency, 0))?;
    token_class.define_method("to_s", method!(RbToken::to_s, 0))?;
    token_class.define_method("inspect", method!(RbToken::inspect, 0))?;

    let dependency_class = module.define_class("TokenDependency", ruby.class_object())?;
    dependency_class.define_method("sentence", method!(RbTokenDependency::sentence, 0))?;
    dependency_class.define_method("bunsetsu", method!(RbTokenDependency::bunsetsu, 0))?;
    dependency_class.define_method("head", method!(RbTokenDependency::head, 0))?;
    dependency_class.define_method("role", method!(RbTokenDependency::role, 0))?;
    dependency_class.define_method("score", method!(RbTokenDependency::score, 0))?;
    dependency_class.define_method("to_hash", method!(RbTokenDependency::to_hash, 0))?;
    dependency_class.define_method("to_h", method!(RbTokenDependency::to_hash, 0))?;
    dependency_class.define_method("inspect", method!(RbTokenDependency::inspect, 0))?;

    Ok(())
}
