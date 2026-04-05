//! Tokenization modes and penalty configurations for PHP.
//!
//! This module defines the different tokenization modes available and their
//! penalty configurations for controlling segmentation behavior.

use ext_php_rs::prelude::*;

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

use crate::error::lindera_value_err;

/// Tokenization mode.
///
/// Determines how text is segmented into tokens.
/// Accepts "normal" or "decompose" (case-insensitive).
#[php_class]
#[php(name = "Lindera\\Mode")]
pub struct PhpMode {
    /// The mode name ("normal" or "decompose").
    mode: String,
}

#[php_impl]
impl PhpMode {
    /// Creates a new Mode instance.
    ///
    /// # Arguments
    ///
    /// * `mode` - Optional mode string ("normal" or "decompose"). Default: "normal".
    ///
    /// # Returns
    ///
    /// A new Mode instance.
    pub fn __construct(mode: Option<String>) -> PhpResult<Self> {
        let mode_str = mode.unwrap_or_else(|| "normal".to_string());
        match mode_str.to_lowercase().as_str() {
            "normal" | "decompose" => Ok(Self {
                mode: mode_str.to_lowercase(),
            }),
            _ => Err(lindera_value_err(format!(
                "Invalid mode: {mode_str}. Must be 'normal' or 'decompose'"
            ))),
        }
    }

    /// Returns the mode name.
    ///
    /// # Returns
    ///
    /// The mode name string.
    #[php(getter)]
    pub fn name(&self) -> String {
        self.mode.clone()
    }

    /// Returns whether this is the normal mode.
    ///
    /// # Returns
    ///
    /// True if the mode is normal.
    pub fn is_normal(&self) -> bool {
        self.mode == "normal"
    }

    /// Returns whether this is the decompose mode.
    ///
    /// # Returns
    ///
    /// True if the mode is decompose.
    pub fn is_decompose(&self) -> bool {
        self.mode == "decompose"
    }

    /// Returns a string representation of the mode.
    ///
    /// # Returns
    ///
    /// The mode name.
    pub fn __to_string(&self) -> String {
        self.mode.clone()
    }
}

impl PhpMode {
    /// Converts this PhpMode to a Lindera Mode.
    ///
    /// # Returns
    ///
    /// The corresponding LinderaMode.
    pub fn to_lindera_mode(&self) -> LinderaMode {
        match self.mode.as_str() {
            "decompose" => LinderaMode::Decompose(LinderaPenalty::default()),
            _ => LinderaMode::Normal,
        }
    }
}

/// Penalty configuration for decompose mode.
///
/// Controls how aggressively compound words are decomposed based on
/// character type and length thresholds.
#[php_class]
#[php(name = "Lindera\\Penalty")]
pub struct PhpPenalty {
    /// Length threshold for kanji penalty.
    kanji_penalty_length_threshold: usize,
    /// Penalty value for kanji sequences exceeding threshold.
    kanji_penalty_length_penalty: i32,
    /// Length threshold for other character penalty.
    other_penalty_length_threshold: usize,
    /// Penalty value for other character sequences exceeding threshold.
    other_penalty_length_penalty: i32,
}

#[php_impl]
impl PhpPenalty {
    /// Creates a new Penalty instance.
    ///
    /// # Arguments
    ///
    /// * `kanji_penalty_length_threshold` - Kanji length threshold (default: 2).
    /// * `kanji_penalty_length_penalty` - Kanji penalty value (default: 3000).
    /// * `other_penalty_length_threshold` - Other character length threshold (default: 7).
    /// * `other_penalty_length_penalty` - Other character penalty value (default: 1700).
    ///
    /// # Returns
    ///
    /// A new Penalty instance.
    pub fn __construct(
        kanji_penalty_length_threshold: Option<i64>,
        kanji_penalty_length_penalty: Option<i64>,
        other_penalty_length_threshold: Option<i64>,
        other_penalty_length_penalty: Option<i64>,
    ) -> Self {
        Self {
            kanji_penalty_length_threshold: kanji_penalty_length_threshold.unwrap_or(2) as usize,
            kanji_penalty_length_penalty: kanji_penalty_length_penalty.unwrap_or(3000) as i32,
            other_penalty_length_threshold: other_penalty_length_threshold.unwrap_or(7) as usize,
            other_penalty_length_penalty: other_penalty_length_penalty.unwrap_or(1700) as i32,
        }
    }

    /// Returns the kanji penalty length threshold.
    ///
    /// # Returns
    ///
    /// The threshold value.
    #[php(getter)]
    pub fn kanji_penalty_length_threshold(&self) -> i64 {
        self.kanji_penalty_length_threshold as i64
    }

    /// Returns the kanji penalty value.
    ///
    /// # Returns
    ///
    /// The penalty value.
    #[php(getter)]
    pub fn kanji_penalty_length_penalty(&self) -> i64 {
        self.kanji_penalty_length_penalty as i64
    }

    /// Returns the other character penalty length threshold.
    ///
    /// # Returns
    ///
    /// The threshold value.
    #[php(getter)]
    pub fn other_penalty_length_threshold(&self) -> i64 {
        self.other_penalty_length_threshold as i64
    }

    /// Returns the other character penalty value.
    ///
    /// # Returns
    ///
    /// The penalty value.
    #[php(getter)]
    pub fn other_penalty_length_penalty(&self) -> i64 {
        self.other_penalty_length_penalty as i64
    }

    /// Returns a string representation of the penalty.
    ///
    /// # Returns
    ///
    /// A string describing the penalty configuration.
    pub fn __to_string(&self) -> String {
        format!(
            "Penalty(kanji_threshold={}, kanji_penalty={}, other_threshold={}, other_penalty={})",
            self.kanji_penalty_length_threshold,
            self.kanji_penalty_length_penalty,
            self.other_penalty_length_threshold,
            self.other_penalty_length_penalty
        )
    }
}

impl From<PhpPenalty> for LinderaPenalty {
    fn from(penalty: PhpPenalty) -> Self {
        LinderaPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

impl From<LinderaPenalty> for PhpPenalty {
    fn from(penalty: LinderaPenalty) -> Self {
        PhpPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lindera::mode::Penalty as LinderaPenalty;

    #[test]
    fn test_phppenalty_to_lindera_penalty() {
        let php_penalty = PhpPenalty {
            kanji_penalty_length_threshold: 5,
            kanji_penalty_length_penalty: 4000,
            other_penalty_length_threshold: 10,
            other_penalty_length_penalty: 2000,
        };
        let lindera_penalty: LinderaPenalty = php_penalty.into();
        assert_eq!(lindera_penalty.kanji_penalty_length_threshold, 5);
        assert_eq!(lindera_penalty.kanji_penalty_length_penalty, 4000);
        assert_eq!(lindera_penalty.other_penalty_length_threshold, 10);
        assert_eq!(lindera_penalty.other_penalty_length_penalty, 2000);
    }

    #[test]
    fn test_lindera_penalty_to_phppenalty() {
        let lindera_penalty = LinderaPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 5000,
            other_penalty_length_threshold: 8,
            other_penalty_length_penalty: 1500,
        };
        let php_penalty: PhpPenalty = lindera_penalty.into();
        assert_eq!(php_penalty.kanji_penalty_length_threshold, 3);
        assert_eq!(php_penalty.kanji_penalty_length_penalty, 5000);
        assert_eq!(php_penalty.other_penalty_length_threshold, 8);
        assert_eq!(php_penalty.other_penalty_length_penalty, 1500);
    }

    #[test]
    fn test_phppenalty_default_values() {
        let php_penalty = PhpPenalty::__construct(None, None, None, None);
        let lindera_penalty: LinderaPenalty = php_penalty.into();
        let default = LinderaPenalty::default();
        assert_eq!(
            lindera_penalty.kanji_penalty_length_threshold,
            default.kanji_penalty_length_threshold
        );
        assert_eq!(
            lindera_penalty.kanji_penalty_length_penalty,
            default.kanji_penalty_length_penalty
        );
        assert_eq!(
            lindera_penalty.other_penalty_length_threshold,
            default.other_penalty_length_threshold
        );
        assert_eq!(
            lindera_penalty.other_penalty_length_penalty,
            default.other_penalty_length_penalty
        );
    }
}
