//! Token class wrapping the Rust Token data for PHP.
//!
//! This module provides the PhpToken class that exposes morphological analysis
//! results to PHP.

use ext_php_rs::boxed::ZBox;
use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendHashTable;

use lindera::token::Token;

use crate::convert::int_arg;

/// Token object wrapping the Rust Token data.
///
/// Provides access to token surface form, byte positions, word ID,
/// and morphological details.
/// The bunsetsu of a token and what that bunsetsu depends on: what the
/// token filter `japanese_dependency` writes on a token.
///
/// The bunsetsu of a sentence are numbered from 0, and each of them but the
/// last depends on a later one, its head.
#[php_class]
#[php(name = "Lindera\\TokenDependency")]
#[derive(Clone)]
pub struct PhpTokenDependency {
    /// The number of the sentence in the text, from 0.
    pub sentence: u32,
    /// The number of the bunsetsu of the token in its sentence, from 0.
    pub bunsetsu: u32,
    /// The number of the bunsetsu the bunsetsu of the token depends on.
    pub head: Option<u32>,
    /// What the token is in its bunsetsu.
    pub role: &'static str,
    /// The score of the decision that attached the bunsetsu to its head.
    pub score: f64,
}

#[php_impl]
impl PhpTokenDependency {
    /// Returns the number of the sentence in the text, from 0.
    #[php(getter, change_case = "snake_case")]
    pub fn sentence(&self) -> i64 {
        i64::from(self.sentence)
    }

    /// Returns the number of the bunsetsu of the token in its sentence,
    /// from 0.
    #[php(getter, change_case = "snake_case")]
    pub fn bunsetsu(&self) -> i64 {
        i64::from(self.bunsetsu)
    }

    /// Returns the number of the bunsetsu that the bunsetsu of the token
    /// depends on, or null for the last bunsetsu of a sentence.
    #[php(getter, change_case = "snake_case")]
    pub fn head(&self) -> Option<i64> {
        self.head.map(i64::from)
    }

    /// Returns what the token is in its bunsetsu: `head`, `function`,
    /// `head_function` or `other`.
    #[php(getter, change_case = "snake_case")]
    pub fn role(&self) -> String {
        self.role.to_string()
    }

    /// Returns the score of the decision that attached the bunsetsu to its
    /// head; 0 where there was nothing to decide.
    #[php(getter, change_case = "snake_case")]
    pub fn score(&self) -> f64 {
        self.score
    }

    /// Returns the dependency as a plain associative array.
    ///
    /// # Returns
    ///
    /// An array with the keys `sentence`, `bunsetsu`, `head` (null for the
    /// last bunsetsu of a sentence), `role` and `score`.
    pub fn to_array(&self) -> PhpResult<ZBox<ZendHashTable>> {
        let mut arr = ZendHashTable::new();
        arr.insert("sentence", i64::from(self.sentence))
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("bunsetsu", i64::from(self.bunsetsu))
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("head", self.head.map(i64::from))
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("role", self.role)
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("score", self.score)
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        Ok(arr)
    }

    /// Returns a string representation of the dependency.
    pub fn __to_string(&self) -> String {
        let head = match self.head {
            Some(head) => head.to_string(),
            None => "null".to_string(),
        };
        format!(
            "TokenDependency(sentence={}, bunsetsu={}, head={}, role='{}', score={})",
            self.sentence, self.bunsetsu, head, self.role, self.score
        )
    }
}

impl PhpTokenDependency {
    /// Creates a `PhpTokenDependency` from a lindera-binding
    /// `DependencyView`.
    ///
    /// # Arguments
    ///
    /// * `view` - The dependency of a token view.
    ///
    /// # Returns
    ///
    /// A new `PhpTokenDependency` instance.
    fn from_view(view: lindera_binding::DependencyView) -> Self {
        Self {
            sentence: view.sentence,
            bunsetsu: view.bunsetsu,
            head: view.head,
            role: view.role(),
            score: view.score,
        }
    }
}

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
    /// The bunsetsu of the token and its head, when the token filter
    /// `japanese_dependency` ran.
    pub dependency: Option<PhpTokenDependency>,
}

#[php_impl]
impl PhpToken {
    /// Returns the surface form of the token.
    ///
    /// # Returns
    ///
    /// The surface string.
    #[php(getter, change_case = "snake_case")]
    pub fn surface(&self) -> String {
        self.surface.clone()
    }

    /// Returns the start byte position in the original text.
    ///
    /// # Returns
    ///
    /// The byte start position.
    #[php(getter, change_case = "snake_case")]
    pub fn byte_start(&self) -> i64 {
        self.byte_start as i64
    }

    /// Returns the end byte position in the original text.
    ///
    /// # Returns
    ///
    /// The byte end position.
    #[php(getter, change_case = "snake_case")]
    pub fn byte_end(&self) -> i64 {
        self.byte_end as i64
    }

    /// Returns the position index of the token.
    ///
    /// # Returns
    ///
    /// The token position.
    #[php(getter, change_case = "snake_case")]
    pub fn position(&self) -> i64 {
        self.position as i64
    }

    /// Returns the word ID in the dictionary.
    ///
    /// # Returns
    ///
    /// The word ID.
    #[php(getter, change_case = "snake_case")]
    pub fn word_id(&self) -> i64 {
        self.word_id as i64
    }

    /// Returns whether this token is an unknown word.
    ///
    /// # Returns
    ///
    /// True if the token is unknown.
    #[php(getter, change_case = "snake_case")]
    pub fn is_unknown(&self) -> bool {
        self.is_unknown
    }

    /// Returns the morphological details of the token.
    ///
    /// # Returns
    ///
    /// A list of detail strings.
    #[php(getter, change_case = "snake_case")]
    pub fn details(&self) -> Vec<String> {
        self.details.clone()
    }

    /// Returns the bunsetsu of the token and its head.
    ///
    /// # Returns
    ///
    /// The dependency, or null unless the token filter
    /// `japanese_dependency` ran.
    #[php(getter, change_case = "snake_case")]
    pub fn dependency(&self) -> Option<PhpTokenDependency> {
        self.dependency.clone()
    }

    /// Returns the detail at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the detail to retrieve, 0 or more.
    ///
    /// # Returns
    ///
    /// The detail string, null if the token has no detail at `index`, or a
    /// `ValueError` for a negative index.
    pub fn get_detail(&self, index: i64) -> PhpResult<Option<String>> {
        let index: usize = int_arg("index", index)?;
        Ok(self.details.get(index).cloned())
    }

    /// Returns the token as an associative array.
    ///
    /// Values keep their natural PHP types (int for the positions and word
    /// id, bool for `is_unknown`, array of string for `details`), so the
    /// result works directly with `json_encode`.
    ///
    /// # Returns
    ///
    /// An array with the keys `surface`, `byte_start`, `byte_end`,
    /// `position`, `word_id`, `is_unknown`, and `details`.
    pub fn to_array(&self) -> PhpResult<ZBox<ZendHashTable>> {
        let mut arr = ZendHashTable::new();
        arr.insert("surface", self.surface.as_str())
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("byte_start", self.byte_start as i64)
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("byte_end", self.byte_end as i64)
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("position", self.position as i64)
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("word_id", self.word_id as i64)
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("is_unknown", self.is_unknown)
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        arr.insert("details", self.details.clone())
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        let dependency = match &self.dependency {
            Some(dependency) => Some(dependency.to_array()?),
            None => None,
        };
        arr.insert("dependency", dependency)
            .map_err(|e| PhpException::from_message(e.to_string()))?;
        Ok(arr)
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
    pub fn from_token(token: Token) -> Self {
        Self::from_view(lindera_binding::TokenView::from_token(token))
    }

    /// Creates a PhpToken from a lindera-binding `TokenView`.
    ///
    /// # Arguments
    ///
    /// * `view` - Token view produced by the lindera-binding tokenizer.
    ///
    /// # Returns
    ///
    /// A new PhpToken instance.
    pub fn from_view(view: lindera_binding::TokenView) -> Self {
        Self {
            surface: view.surface,
            byte_start: view.byte_start,
            byte_end: view.byte_end,
            position: view.position,
            word_id: view.word_id,
            is_unknown: view.is_unknown,
            details: view.details,
            dependency: view.dependency.map(PhpTokenDependency::from_view),
        }
    }
}
