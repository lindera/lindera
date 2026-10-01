//! Whitespace classification by the dictionary's `SPACE` character category.
//!
//! MeCab treats the characters `char.def` puts in the `SPACE` category (the
//! category of U+0020, which `char.def` files reserve for it) as whitespace:
//! it skips them in the lattice and does not output them. Lindera uses the
//! same set for [`LatticeOptions::skip_whitespace`], for the left-space
//! penalty and for dropping whitespace tokens when `keep_whitespace` is
//! false.
//!
//! [`LatticeOptions::skip_whitespace`]: crate::viterbi::LatticeOptions::skip_whitespace

use crate::dictionary::character_definition::{CategoryId, CharacterDefinition};

/// Precomputed membership test for a dictionary's `SPACE` category.
///
/// Built once per dictionary so the per-character check in the lattice and
/// the segmenter is one indexed load for ASCII/Latin-1, which covers every
/// character real `char.def` files put in `SPACE` (0x20, 0x09, 0x0A, 0x0B,
/// 0x0D, and IPADIC's 0xD0). When the category has no member above U+00FF,
/// as in every bundled dictionary, other characters are answered without a
/// lookup; otherwise they fall back to a category lookup.
#[derive(Clone, Debug)]
pub struct WhitespaceClassifier {
    /// `SPACE`-category membership for the codepoints below 256.
    ascii: [bool; 256],
    /// The dictionary's `SPACE` category, for codepoints outside `ascii`.
    category: CategoryId,
    /// Whether any codepoint above U+00FF is in the category.
    above_latin1: bool,
}

impl WhitespaceClassifier {
    /// Builds the classifier for a dictionary's character definitions.
    ///
    /// # Arguments
    ///
    /// * `char_definitions` - The dictionary's character definitions.
    ///
    /// # Returns
    ///
    /// The classifier, or `None` when `char.def` defines no `SPACE`
    /// category.
    pub fn new(char_definitions: &CharacterDefinition) -> Option<Self> {
        let category = char_definitions.category_id_by_name("SPACE")?;
        let mut ascii = [false; 256];
        for (codepoint, is_space) in ascii.iter_mut().enumerate() {
            if let Some(c) = char::from_u32(codepoint as u32) {
                *is_space = char_definitions.lookup_categories(c).contains(&category);
            }
        }
        let above_latin1 = char_definitions.has_category_from(category, 256);
        Some(Self {
            ascii,
            category,
            above_latin1,
        })
    }

    /// Returns the `SPACE` category id the classifier tests for.
    ///
    /// # Returns
    ///
    /// The category id.
    #[inline]
    pub fn category(&self) -> CategoryId {
        self.category
    }

    /// Returns the precomputed answer for a codepoint below 256.
    ///
    /// # Arguments
    ///
    /// * `codepoint` - A codepoint below 256.
    ///
    /// # Returns
    ///
    /// `true` for a `SPACE`-category character.
    #[inline]
    pub(crate) fn is_space_below_256(&self, codepoint: u32) -> bool {
        self.ascii[codepoint as usize]
    }

    /// Returns whether any codepoint above U+00FF is in the category; when
    /// not, [`Self::is_space`] answers those codepoints without a lookup.
    ///
    /// # Returns
    ///
    /// `true` when the category has members above U+00FF.
    #[inline]
    pub(crate) fn has_members_above_latin1(&self) -> bool {
        self.above_latin1
    }

    /// Returns whether `c` is in the `SPACE` category.
    ///
    /// # Arguments
    ///
    /// * `c` - The character to classify.
    /// * `char_definitions` - The character definitions the classifier was
    ///   built from, read only for codepoints of 256 and above.
    ///
    /// # Returns
    ///
    /// `true` for a `SPACE`-category character.
    #[inline]
    pub fn is_space(&self, c: char, char_definitions: &CharacterDefinition) -> bool {
        let codepoint = c as u32;
        if codepoint < 256 {
            self.ascii[codepoint as usize]
        } else {
            self.above_latin1
                && char_definitions
                    .lookup_categories(c)
                    .contains(&self.category)
        }
    }
}
