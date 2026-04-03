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
