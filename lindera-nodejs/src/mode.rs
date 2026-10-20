//! Tokenization modes and penalty configurations.
//!
//! This module defines the different tokenization modes available and their
//! penalty configurations for controlling segmentation behavior. A `Mode`
//! holds the penalty of decompose mode, as the Rust `lindera::mode::Mode`
//! does, and is what `TokenizerBuilder.setMode` and the `Tokenizer`
//! constructor take (#1218).

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};
use napi::bindgen_prelude::{FromNapiValue, ValidateNapiValue};
use napi::sys;

use crate::error::to_napi_error;
use crate::util::optional_integer_arg;

/// Tokenization mode: normal, or decompose with a `Penalty`.
///
/// The JS counterpart of the Rust `lindera::mode::Mode`. It has no
/// constructor; `Mode.normal()`, `Mode.decompose()` and `Mode.fromStr()`
/// create one. A mode cannot be changed, and `equals` compares its name and
/// penalty.
// The Rust name is `Mode`, not `JsMode`: napi-rs also exports a class `JsX`
// with `js_name = "X"` as `JsX`.
#[napi]
#[derive(Debug, Clone, PartialEq)]
pub struct Mode {
    /// The mode, with its penalty for decompose mode.
    inner: LinderaMode,
}

impl Mode {
    /// Returns the wrapped `lindera::mode::Mode`.
    pub fn as_lindera(&self) -> &LinderaMode {
        &self.inner
    }

    /// Creates decompose mode with `penalty`, or with the default penalty for
    /// `None`.
    fn decompose_with(penalty: Option<LinderaPenalty>) -> Self {
        Self {
            inner: LinderaMode::Decompose(penalty.unwrap_or_default()),
        }
    }
}

#[napi]
impl Mode {
    /// Creates normal mode.
    ///
    /// # Returns
    ///
    /// Normal mode.
    #[napi(factory)]
    pub fn normal() -> Self {
        Self {
            inner: LinderaMode::Normal,
        }
    }

    /// Creates decompose mode.
    ///
    /// # Arguments
    ///
    /// * `penalty` - The `Penalty`. `null` or `undefined` (the default) uses
    ///   the default penalty; a field left out keeps its default.
    ///
    /// # Returns
    ///
    /// Decompose mode with the penalty. Throws, with code `InvalidArg`, for
    /// a penalty that is not an object or a field that is not an integer in
    /// its range.
    #[napi(factory)]
    pub fn decompose(
        #[napi(ts_arg_type = "Penalty | undefined | null")] penalty: Option<PenaltyArg>,
    ) -> napi::Result<Self> {
        Ok(Self::decompose_with(optional_penalty(penalty)?))
    }

    /// Creates a mode from its name, as the Rust `Mode` parses one.
    ///
    /// # Arguments
    ///
    /// * `name` - `"normal"` or `"decompose"`, case sensitive.
    ///
    /// # Returns
    ///
    /// The mode, with the default penalty for `"decompose"`. Throws for any
    /// other name, as `setMode` did for it in v6.
    #[napi(factory, js_name = "fromStr")]
    pub fn parse(name: String) -> napi::Result<Self> {
        let inner = lindera_binding::parse_mode(&name).map_err(to_napi_error)?;
        Ok(Self { inner })
    }

    /// The name of the mode: `"normal"` or `"decompose"`.
    #[napi(getter)]
    pub fn name(&self) -> String {
        self.inner.as_str().to_string()
    }

    /// The penalty of decompose mode, as a copy with every field set, or
    /// `null` for normal mode.
    #[napi(getter)]
    pub fn penalty(&self) -> Option<Penalty> {
        match &self.inner {
            LinderaMode::Normal => None,
            LinderaMode::Decompose(penalty) => Some(penalty.clone().into()),
        }
    }

    /// Returns whether this is normal mode.
    #[napi]
    pub fn is_normal(&self) -> bool {
        matches!(self.inner, LinderaMode::Normal)
    }

    /// Returns whether this is decompose mode.
    #[napi]
    pub fn is_decompose(&self) -> bool {
        matches!(self.inner, LinderaMode::Decompose(_))
    }

    /// Returns whether `other` has the same name and penalty.
    ///
    /// # Arguments
    ///
    /// * `other` - Another `Mode`. A value that is not a `Mode` throws.
    #[napi]
    pub fn equals(&self, other: &Mode) -> bool {
        self == other
    }

    /// Returns the name of the mode, as `name` does.
    #[napi(js_name = "toString")]
    pub fn js_to_string(&self) -> String {
        self.name()
    }
}

/// Penalty configuration for decompose mode.
///
/// Controls how aggressively compound words are decomposed based on
/// character type and length thresholds. Pass it to `Mode.decompose`. A
/// field left out keeps its default.
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

impl From<LinderaPenalty> for Penalty {
    /// Converts a core penalty to a JS penalty with every field set.
    fn from(penalty: LinderaPenalty) -> Self {
        Self {
            kanji_penalty_length_threshold: Some(penalty.kanji_penalty_length_threshold as f64),
            kanji_penalty_length_penalty: Some(penalty.kanji_penalty_length_penalty.into()),
            other_penalty_length_threshold: Some(penalty.other_penalty_length_threshold as f64),
            other_penalty_length_penalty: Some(penalty.other_penalty_length_penalty.into()),
        }
    }
}

/// The penalty argument of `Mode.decompose`.
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

/// Converts the optional penalty argument of `Mode.decompose`.
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
    fn test_mode_factories() {
        assert_eq!(Mode::normal().as_lindera(), &LinderaMode::Normal);
        assert_eq!(
            Mode::decompose_with(None).as_lindera(),
            &LinderaMode::Decompose(LinderaPenalty::default())
        );
        let penalty = LinderaPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 0,
            other_penalty_length_threshold: 4,
            other_penalty_length_penalty: 3000,
        };
        let mode = Mode::decompose_with(Some(penalty.clone()));
        assert_eq!(mode.as_lindera(), &LinderaMode::Decompose(penalty.clone()));
        let back = match mode.penalty().map(LinderaPenalty::try_from) {
            Some(Ok(back)) => back,
            other => panic!("decompose mode gave the penalty {other:?}"),
        };
        assert_eq!(back, penalty);
        assert!(Mode::normal().penalty().is_none());
    }

    #[test]
    fn test_mode_names_kinds_and_equality() {
        let normal = Mode::normal();
        let decompose = Mode::decompose_with(None);
        assert_eq!(normal.name(), "normal");
        assert_eq!(decompose.js_to_string(), "decompose");
        assert!(normal.is_normal() && !normal.is_decompose());
        assert!(decompose.is_decompose() && !decompose.is_normal());
        assert!(decompose.equals(&Mode::decompose_with(None)));
        assert!(
            !decompose.equals(&Mode::decompose_with(Some(LinderaPenalty {
                kanji_penalty_length_penalty: 0,
                ..LinderaPenalty::default()
            })))
        );
        assert!(!normal.equals(&decompose));
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
