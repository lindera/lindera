//! Tokenization modes and penalty configurations.
//!
//! This module defines the different tokenization modes available and their
//! penalty configurations for controlling segmentation behavior.

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

/// Tokenization mode.
///
/// Determines how text is segmented into tokens.
///
/// - `Normal`: Standard tokenization based on dictionary cost.
/// - `Decompose`: Decomposes compound words using penalty-based segmentation.
#[napi(string_enum)]
pub enum JsMode {
    /// Standard tokenization based on dictionary cost
    Normal,
    /// Decompose compound words using penalty-based segmentation
    Decompose,
}

impl From<JsMode> for LinderaMode {
    fn from(mode: JsMode) -> Self {
        match mode {
            JsMode::Normal => LinderaMode::Normal,
            JsMode::Decompose => LinderaMode::Decompose(LinderaPenalty::default()),
        }
    }
}

impl From<LinderaMode> for JsMode {
    fn from(mode: LinderaMode) -> Self {
        match mode {
            LinderaMode::Normal => JsMode::Normal,
            LinderaMode::Decompose(_) => JsMode::Decompose,
        }
    }
}

/// Penalty configuration for decompose mode.
///
/// Controls how aggressively compound words are decomposed based on
/// character type and length thresholds.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct JsPenalty {
    /// Length threshold for kanji sequences before applying penalty (default: 2).
    pub kanji_penalty_length_threshold: u32,
    /// Penalty value for long kanji sequences (default: 3000).
    pub kanji_penalty_length_penalty: i32,
    /// Length threshold for other character sequences before applying penalty (default: 7).
    pub other_penalty_length_threshold: u32,
    /// Penalty value for long other-character sequences (default: 1700).
    pub other_penalty_length_penalty: i32,
}

impl From<JsPenalty> for LinderaPenalty {
    fn from(penalty: JsPenalty) -> Self {
        LinderaPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold as usize,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold as usize,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

impl From<LinderaPenalty> for JsPenalty {
    fn from(penalty: LinderaPenalty) -> Self {
        JsPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold as u32,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold as u32,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_mode_normal_to_lindera_mode() {
        let lindera_mode: LinderaMode = JsMode::Normal.into();
        assert!(matches!(lindera_mode, LinderaMode::Normal));
    }

    #[test]
    fn test_js_mode_decompose_to_lindera_mode() {
        let lindera_mode: LinderaMode = JsMode::Decompose.into();
        assert!(matches!(lindera_mode, LinderaMode::Decompose(_)));
    }

    #[test]
    fn test_lindera_mode_normal_to_js_mode() {
        let js_mode: JsMode = LinderaMode::Normal.into();
        assert!(matches!(js_mode, JsMode::Normal));
    }

    #[test]
    fn test_lindera_mode_decompose_to_js_mode() {
        let penalty = LinderaPenalty::default();
        let js_mode: JsMode = LinderaMode::Decompose(penalty).into();
        assert!(matches!(js_mode, JsMode::Decompose));
    }

    #[test]
    fn test_js_penalty_to_lindera_penalty() {
        let js_penalty = JsPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 5000,
            other_penalty_length_threshold: 10,
            other_penalty_length_penalty: 2000,
        };
        let lindera_penalty: LinderaPenalty = js_penalty.into();
        assert_eq!(lindera_penalty.kanji_penalty_length_threshold, 3);
        assert_eq!(lindera_penalty.kanji_penalty_length_penalty, 5000);
        assert_eq!(lindera_penalty.other_penalty_length_threshold, 10);
        assert_eq!(lindera_penalty.other_penalty_length_penalty, 2000);
    }

    #[test]
    fn test_lindera_penalty_to_js_penalty() {
        let lindera_penalty = LinderaPenalty {
            kanji_penalty_length_threshold: 4,
            kanji_penalty_length_penalty: 6000,
            other_penalty_length_threshold: 8,
            other_penalty_length_penalty: 1500,
        };
        let js_penalty: JsPenalty = lindera_penalty.into();
        assert_eq!(js_penalty.kanji_penalty_length_threshold, 4);
        assert_eq!(js_penalty.kanji_penalty_length_penalty, 6000);
        assert_eq!(js_penalty.other_penalty_length_threshold, 8);
        assert_eq!(js_penalty.other_penalty_length_penalty, 1500);
    }

    #[test]
    fn test_penalty_roundtrip() {
        let original = JsPenalty {
            kanji_penalty_length_threshold: 2,
            kanji_penalty_length_penalty: 3000,
            other_penalty_length_threshold: 7,
            other_penalty_length_penalty: 1700,
        };
        let lindera: LinderaPenalty = original.clone().into();
        let roundtripped: JsPenalty = lindera.into();
        assert_eq!(
            roundtripped.kanji_penalty_length_threshold,
            original.kanji_penalty_length_threshold
        );
        assert_eq!(
            roundtripped.kanji_penalty_length_penalty,
            original.kanji_penalty_length_penalty
        );
        assert_eq!(
            roundtripped.other_penalty_length_threshold,
            original.other_penalty_length_threshold
        );
        assert_eq!(
            roundtripped.other_penalty_length_penalty,
            original.other_penalty_length_penalty
        );
    }
}
