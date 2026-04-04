use std::str::FromStr;

use wasm_bindgen::prelude::*;

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

/// Tokenization mode.
///
/// Determines how text is segmented into tokens.
#[wasm_bindgen(js_name = "Mode")]
#[derive(Debug, Clone, Copy)]
pub enum JsMode {
    /// Standard tokenization based on dictionary cost
    Normal,
    /// Decompose compound words using penalty-based segmentation
    Decompose,
}

#[wasm_bindgen]
impl JsMode {}

impl FromStr for JsMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(JsMode::Normal),
            "decompose" => Ok(JsMode::Decompose),
            _ => Err(format!("Invalid mode: {s}")),
        }
    }
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
#[wasm_bindgen(js_name = "Penalty")]
#[derive(Debug, Clone, Copy)]
pub struct JsPenalty {
    pub kanji_penalty_length_threshold: usize,
    pub kanji_penalty_length_penalty: i32,
    pub other_penalty_length_threshold: usize,
    pub other_penalty_length_penalty: i32,
}

#[wasm_bindgen]
impl JsPenalty {
    #[wasm_bindgen(constructor)]
    pub fn new(
        kanji_penalty_length_threshold: Option<usize>,
        kanji_penalty_length_penalty: Option<i32>,
        other_penalty_length_threshold: Option<usize>,
        other_penalty_length_penalty: Option<i32>,
    ) -> Self {
        JsPenalty {
            kanji_penalty_length_threshold: kanji_penalty_length_threshold.unwrap_or(2),
            kanji_penalty_length_penalty: kanji_penalty_length_penalty.unwrap_or(3000),
            other_penalty_length_threshold: other_penalty_length_threshold.unwrap_or(7),
            other_penalty_length_penalty: other_penalty_length_penalty.unwrap_or(1700),
        }
    }
}

impl From<JsPenalty> for LinderaPenalty {
    fn from(penalty: JsPenalty) -> Self {
        LinderaPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

impl From<LinderaPenalty> for JsPenalty {
    fn from(penalty: LinderaPenalty) -> Self {
        JsPenalty {
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

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_penalty_new_defaults_wasm() {
        let penalty = JsPenalty::new(None, None, None, None);

        assert_eq!(penalty.kanji_penalty_length_threshold, 2);
        assert_eq!(penalty.kanji_penalty_length_penalty, 3000);
        assert_eq!(penalty.other_penalty_length_threshold, 7);
        assert_eq!(penalty.other_penalty_length_penalty, 1700);
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_penalty_new_custom_wasm() {
        let penalty = JsPenalty::new(Some(3), Some(5000), Some(10), Some(2000));

        assert_eq!(penalty.kanji_penalty_length_threshold, 3);
        assert_eq!(penalty.kanji_penalty_length_penalty, 5000);
        assert_eq!(penalty.other_penalty_length_threshold, 10);
        assert_eq!(penalty.other_penalty_length_penalty, 2000);
    }

    #[test]
    fn test_penalty_new_defaults() {
        let penalty = JsPenalty::new(None, None, None, None);

        assert_eq!(penalty.kanji_penalty_length_threshold, 2);
        assert_eq!(penalty.kanji_penalty_length_penalty, 3000);
        assert_eq!(penalty.other_penalty_length_threshold, 7);
        assert_eq!(penalty.other_penalty_length_penalty, 1700);
    }

    #[test]
    fn test_penalty_new_custom() {
        let penalty = JsPenalty::new(Some(3), Some(5000), Some(10), Some(2000));

        assert_eq!(penalty.kanji_penalty_length_threshold, 3);
        assert_eq!(penalty.kanji_penalty_length_penalty, 5000);
        assert_eq!(penalty.other_penalty_length_threshold, 10);
        assert_eq!(penalty.other_penalty_length_penalty, 2000);
    }

    #[test]
    fn test_js_mode_from_str() {
        assert!(matches!(JsMode::from_str("normal"), Ok(JsMode::Normal)));
        assert!(matches!(
            JsMode::from_str("decompose"),
            Ok(JsMode::Decompose)
        ));
        assert!(JsMode::from_str("invalid").is_err());
    }

    #[test]
    fn test_js_mode_to_lindera_mode() {
        let normal: LinderaMode = JsMode::Normal.into();
        assert!(matches!(normal, LinderaMode::Normal));

        let decompose: LinderaMode = JsMode::Decompose.into();
        assert!(matches!(decompose, LinderaMode::Decompose(_)));
    }

    #[test]
    fn test_lindera_mode_to_js_mode() {
        let normal: JsMode = LinderaMode::Normal.into();
        assert!(matches!(normal, JsMode::Normal));

        let decompose: JsMode = LinderaMode::Decompose(LinderaPenalty::default()).into();
        assert!(matches!(decompose, JsMode::Decompose));
    }

    #[test]
    fn test_js_penalty_to_lindera_penalty() {
        let js_penalty = JsPenalty::new(Some(5), Some(4000), Some(8), Some(1500));
        let lindera_penalty: LinderaPenalty = js_penalty.into();

        assert_eq!(lindera_penalty.kanji_penalty_length_threshold, 5);
        assert_eq!(lindera_penalty.kanji_penalty_length_penalty, 4000);
        assert_eq!(lindera_penalty.other_penalty_length_threshold, 8);
        assert_eq!(lindera_penalty.other_penalty_length_penalty, 1500);
    }

    #[test]
    fn test_lindera_penalty_to_js_penalty() {
        let lindera_penalty = LinderaPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 2500,
            other_penalty_length_threshold: 6,
            other_penalty_length_penalty: 1200,
        };
        let js_penalty: JsPenalty = lindera_penalty.into();

        assert_eq!(js_penalty.kanji_penalty_length_threshold, 3);
        assert_eq!(js_penalty.kanji_penalty_length_penalty, 2500);
        assert_eq!(js_penalty.other_penalty_length_threshold, 6);
        assert_eq!(js_penalty.other_penalty_length_penalty, 1200);
    }
}
