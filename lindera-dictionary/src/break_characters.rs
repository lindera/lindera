//! Break characters: characters that a token does not run across (#1353).
//!
//! The bundled dictionaries put `・` (U+30FB) in the katakana category of
//! their `char.def`, so an unknown katakana word runs through it, and
//! whether text is split at `・` depends on whether its first word is in the
//! dictionary: MeCab, and Lindera with no break characters, split
//! `ジョン・レノン` after the dictionary word `ジョン` but keep the unknown
//! `レノン・ジョン` whole. With break characters, a boundary between a break
//! character and a character that is not one ends every unknown word, in
//! every mode, and, in Decompose mode, every system-dictionary word (see
//! [`LatticeOptions::break_characters`]).
//!
//! [`LatticeOptions::break_characters`]: crate::viterbi::LatticeOptions::break_characters

use std::fmt;

/// The break characters of a segmenter that is not told otherwise: `・`
/// (U+30FB KATAKANA MIDDLE DOT).
///
/// `･` (U+FF65) is not among them: every bundled dictionary puts it in the
/// symbol category, apart from the halfwidth katakana, so no unknown word
/// runs through it anyway.
pub const DEFAULT_BREAK_CHARACTERS: &str = "\u{30FB}";

/// A set of break characters.
///
/// An empty set breaks nothing, which gives MeCab's segmentation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BreakCharacters {
    /// The characters, sorted and each once.
    chars: Vec<char>,
}

impl BreakCharacters {
    /// Builds the set of the characters of `chars`.
    ///
    /// # Arguments
    ///
    /// * `chars` - The break characters; repeated ones count once, and an
    ///   empty string gives the empty set.
    ///
    /// # Returns
    ///
    /// The set.
    pub fn new(chars: &str) -> Self {
        let mut chars: Vec<char> = chars.chars().collect();
        chars.sort_unstable();
        chars.dedup();
        Self { chars }
    }

    /// Returns the default set, [`DEFAULT_BREAK_CHARACTERS`].
    ///
    /// # Returns
    ///
    /// The set of `・`.
    pub fn default_set() -> Self {
        Self::new(DEFAULT_BREAK_CHARACTERS)
    }

    /// Returns whether `c` is a break character.
    ///
    /// # Arguments
    ///
    /// * `c` - The character.
    ///
    /// # Returns
    ///
    /// `true` when `c` is in the set.
    #[inline]
    pub fn contains(&self, c: char) -> bool {
        // A set holds a handful of characters at most (the default holds
        // one), so a scan beats a search.
        self.chars.contains(&c)
    }

    /// Returns whether the set is empty, which turns the rule off.
    ///
    /// # Returns
    ///
    /// `true` when no character is a break character.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }
}

/// Writes the characters in order, each once, as one string: the form the
/// configuration and the CLI take.
impl fmt::Display for BreakCharacters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &c in &self.chars {
            write!(f, "{c}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_holds_each_character_once() {
        let set = BreakCharacters::new("・･・");
        assert!(set.contains('・'));
        assert!(set.contains('･'));
        assert!(!set.contains('ン'));
        assert_eq!(set.to_string(), "・･");
        assert_eq!(set, BreakCharacters::new("･・"));
    }

    #[test]
    fn the_default_set_is_the_middle_dot() {
        let set = BreakCharacters::default_set();
        assert_eq!(set.to_string(), "・");
        assert!(set.contains('\u{30FB}'));
        assert!(!set.contains('\u{FF65}'));
    }

    #[test]
    fn an_empty_string_gives_the_empty_set() {
        assert!(BreakCharacters::new("").is_empty());
        assert_eq!(BreakCharacters::new(""), BreakCharacters::default());
        assert!(!BreakCharacters::default_set().is_empty());
    }
}
