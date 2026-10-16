//! Tokenization modes and penalty configurations.
//!
//! This module defines the different tokenization modes available and their
//! penalty configurations for controlling segmentation behavior.

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};
use napi::bindgen_prelude::{FromNapiValue, ValidateNapiValue};
use napi::sys;

use crate::util::optional_integer_arg;

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
/// character type and length thresholds. Pass it with "decompose" to
/// `TokenizerBuilder.setMode` or to the `Tokenizer` constructor. A field
/// left out keeps its default.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct Penalty {
    /// Length threshold for kanji sequences before applying the penalty, an
    /// integer 0 or more (default: 2).
    pub kanji_penalty_length_threshold: Option<f64>,
    /// Penalty value for long kanji sequences, a 32-bit signed integer
    /// (default: 3000).
    pub kanji_penalty_length_penalty: Option<f64>,
    /// Length threshold for other character sequences before applying the
    /// penalty, an integer 0 or more (default: 7).
    pub other_penalty_length_threshold: Option<f64>,
    /// Penalty value for long other-character sequences, a 32-bit signed
    /// integer (default: 1700).
    pub other_penalty_length_penalty: Option<f64>,
}

impl TryFrom<Penalty> for LinderaPenalty {
    type Error = napi::Error;

    /// Checks each field of a JS penalty and fills the ones left out with
    /// the defaults. A JS number is taken as `f64` and checked, because
    /// napi-rs would wrap a value outside the range of the field (#1209).
    ///
    /// # Arguments
    ///
    /// * `penalty` - The penalty passed from JS.
    ///
    /// # Returns
    ///
    /// The core penalty, or an `InvalidArg` error that names the first field
    /// that is not an integer in its range.
    fn try_from(penalty: Penalty) -> napi::Result<Self> {
        let default = LinderaPenalty::default();
        Ok(LinderaPenalty {
            kanji_penalty_length_threshold: optional_integer_arg(
                "kanjiPenaltyLengthThreshold",
                penalty.kanji_penalty_length_threshold,
            )?
            .unwrap_or(default.kanji_penalty_length_threshold),
            kanji_penalty_length_penalty: optional_integer_arg(
                "kanjiPenaltyLengthPenalty",
                penalty.kanji_penalty_length_penalty,
            )?
            .unwrap_or(default.kanji_penalty_length_penalty),
            other_penalty_length_threshold: optional_integer_arg(
                "otherPenaltyLengthThreshold",
                penalty.other_penalty_length_threshold,
            )?
            .unwrap_or(default.other_penalty_length_threshold),
            other_penalty_length_penalty: optional_integer_arg(
                "otherPenaltyLengthPenalty",
                penalty.other_penalty_length_penalty,
            )?
            .unwrap_or(default.other_penalty_length_penalty),
        })
    }
}

/// The penalty argument of `setMode` and the `Tokenizer` constructor.
///
/// napi-rs reads a `#[napi(object)]` argument without checking that the
/// value is an object, so a number or a string would become a [`Penalty`]
/// with every field left out, that is the default penalty. This type checks
/// the value with napi-rs's own validation of [`Penalty`] first, as
/// `#[napi(strict)]` would for every argument of a function.
pub struct PenaltyArg(pub Penalty);

impl FromNapiValue for PenaltyArg {
    /// Converts a JS value to a [`Penalty`], failing for a value that is not
    /// an object.
    ///
    /// # Arguments
    ///
    /// * `env` - The environment of the current call.
    /// * `napi_val` - The JS value to convert.
    ///
    /// # Returns
    ///
    /// The penalty, an `InvalidArg` error for a value that is not an object,
    /// or the error napi-rs raises for a field that is not a number.
    unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
        // SAFETY: napi-rs calls this with the environment and a value handle
        // of the current call, which is what both calls require.
        unsafe {
            <Penalty as ValidateNapiValue>::validate(env, napi_val)?;
            Penalty::from_napi_value(env, napi_val).map(PenaltyArg)
        }
    }
}

/// Converts the optional penalty argument of `setMode` and the `Tokenizer`
/// constructor.
///
/// # Arguments
///
/// * `penalty` - The penalty passed from JS, or `None` for `null` or
///   `undefined`.
///
/// # Returns
///
/// `None` when no penalty is passed, the core penalty otherwise, or the
/// error of the conversion.
pub fn optional_penalty(penalty: Option<PenaltyArg>) -> napi::Result<Option<LinderaPenalty>> {
    penalty
        .map(|PenaltyArg(penalty)| LinderaPenalty::try_from(penalty))
        .transpose()
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
    fn test_penalty_to_lindera_penalty() {
        let penalty = Penalty {
            kanji_penalty_length_threshold: Some(3.0),
            kanji_penalty_length_penalty: Some(-5000.0),
            other_penalty_length_threshold: Some(10.0),
            other_penalty_length_penalty: Some(0.0),
        };
        let lindera_penalty = match LinderaPenalty::try_from(penalty) {
            Ok(penalty) => penalty,
            Err(err) => panic!("conversion failed: {err}"),
        };
        assert_eq!(
            lindera_penalty,
            LinderaPenalty {
                kanji_penalty_length_threshold: 3,
                kanji_penalty_length_penalty: -5000,
                other_penalty_length_threshold: 10,
                other_penalty_length_penalty: 0,
            }
        );
    }

    #[test]
    fn test_penalty_fields_left_out_keep_their_defaults() {
        let lindera_penalty = match LinderaPenalty::try_from(Penalty {
            kanji_penalty_length_penalty: Some(0.0),
            ..Penalty::default()
        }) {
            Ok(penalty) => penalty,
            Err(err) => panic!("conversion failed: {err}"),
        };
        assert_eq!(
            lindera_penalty,
            LinderaPenalty {
                kanji_penalty_length_penalty: 0,
                ..LinderaPenalty::default()
            }
        );
    }

    #[test]
    fn test_penalty_field_outside_its_range_is_an_error() {
        for (penalty, name) in [
            (
                Penalty {
                    kanji_penalty_length_threshold: Some(-1.0),
                    ..Penalty::default()
                },
                "kanjiPenaltyLengthThreshold",
            ),
            (
                Penalty {
                    kanji_penalty_length_penalty: Some(2_147_483_648.0),
                    ..Penalty::default()
                },
                "kanjiPenaltyLengthPenalty",
            ),
            (
                Penalty {
                    other_penalty_length_threshold: Some(1.5),
                    ..Penalty::default()
                },
                "otherPenaltyLengthThreshold",
            ),
            (
                Penalty {
                    other_penalty_length_penalty: Some(f64::NAN),
                    ..Penalty::default()
                },
                "otherPenaltyLengthPenalty",
            ),
        ] {
            match LinderaPenalty::try_from(penalty) {
                Ok(penalty) => panic!("{name}: converted to {penalty:?}"),
                Err(err) => {
                    assert_eq!(err.status, napi::Status::InvalidArg, "{name}");
                    assert!(err.reason.contains(name), "{name}: {}", err.reason);
                }
            }
        }
    }

    #[test]
    fn test_optional_penalty() {
        assert_eq!(optional_penalty(None).ok(), Some(None));
        assert_eq!(
            optional_penalty(Some(PenaltyArg(Penalty::default()))).ok(),
            Some(Some(LinderaPenalty::default()))
        );
    }
}
