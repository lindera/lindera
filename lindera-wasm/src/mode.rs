use std::str::FromStr;

use wasm_bindgen::prelude::*;

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

use crate::argument::{integer_arg, optional_integer_arg};

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
    /// Length threshold for kanji sequences before applying the penalty.
    kanji_penalty_length_threshold: usize,
    /// Penalty value for long kanji sequences.
    kanji_penalty_length_penalty: i32,
    /// Length threshold for other character sequences before applying the
    /// penalty.
    other_penalty_length_threshold: usize,
    /// Penalty value for long other-character sequences.
    other_penalty_length_penalty: i32,
}

// The fields are private, with the getters and setters written out: the
// setters wasm-bindgen generates for `pub` fields take the number through
// `>>> 0` / `>> 0`, which wraps a value outside the field's range
// (lindera/lindera#1209).
#[wasm_bindgen(js_class = "Penalty")]
impl JsPenalty {
    /// Creates a penalty configuration.
    ///
    /// # Arguments
    ///
    /// * `kanji_penalty_length_threshold` - Kanji length threshold, an integer
    ///   of 0 or more (default: 2).
    /// * `kanji_penalty_length_penalty` - Kanji penalty value, a 32-bit signed
    ///   integer (default: 3000).
    /// * `other_penalty_length_threshold` - Other character length threshold,
    ///   an integer of 0 or more (default: 7).
    /// * `other_penalty_length_penalty` - Other character penalty value, a
    ///   32-bit signed integer (default: 1700).
    ///
    /// # Returns
    ///
    /// The penalty, or a thrown string for a value that is not an integer in
    /// its range.
    #[wasm_bindgen(constructor)]
    pub fn new(
        kanji_penalty_length_threshold: Option<f64>,
        kanji_penalty_length_penalty: Option<f64>,
        other_penalty_length_threshold: Option<f64>,
        other_penalty_length_penalty: Option<f64>,
    ) -> Result<JsPenalty, JsValue> {
        let default = LinderaPenalty::default();
        Ok(JsPenalty {
            kanji_penalty_length_threshold: optional_integer_arg(
                "kanji_penalty_length_threshold",
                kanji_penalty_length_threshold,
            )?
            .unwrap_or(default.kanji_penalty_length_threshold),
            kanji_penalty_length_penalty: optional_integer_arg(
                "kanji_penalty_length_penalty",
                kanji_penalty_length_penalty,
            )?
            .unwrap_or(default.kanji_penalty_length_penalty),
            other_penalty_length_threshold: optional_integer_arg(
                "other_penalty_length_threshold",
                other_penalty_length_threshold,
            )?
            .unwrap_or(default.other_penalty_length_threshold),
            other_penalty_length_penalty: optional_integer_arg(
                "other_penalty_length_penalty",
                other_penalty_length_penalty,
            )?
            .unwrap_or(default.other_penalty_length_penalty),
        })
    }

    /// Length threshold for kanji sequences before applying the penalty.
    #[wasm_bindgen(getter)]
    pub fn kanji_penalty_length_threshold(&self) -> usize {
        self.kanji_penalty_length_threshold
    }

    /// Sets the kanji length threshold.
    ///
    /// # Arguments
    ///
    /// * `value` - An integer of 0 or more.
    ///
    /// # Returns
    ///
    /// Nothing, or a thrown string for any other number, which leaves the
    /// threshold unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kanji_penalty_length_threshold(&mut self, value: f64) -> Result<(), JsValue> {
        self.kanji_penalty_length_threshold = integer_arg("kanji_penalty_length_threshold", value)?;
        Ok(())
    }

    /// Penalty value for long kanji sequences.
    #[wasm_bindgen(getter)]
    pub fn kanji_penalty_length_penalty(&self) -> i32 {
        self.kanji_penalty_length_penalty
    }

    /// Sets the kanji penalty value.
    ///
    /// # Arguments
    ///
    /// * `value` - A 32-bit signed integer.
    ///
    /// # Returns
    ///
    /// Nothing, or a thrown string for any other number, which leaves the
    /// penalty unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kanji_penalty_length_penalty(&mut self, value: f64) -> Result<(), JsValue> {
        self.kanji_penalty_length_penalty = integer_arg("kanji_penalty_length_penalty", value)?;
        Ok(())
    }

    /// Length threshold for other character sequences before applying the
    /// penalty.
    #[wasm_bindgen(getter)]
    pub fn other_penalty_length_threshold(&self) -> usize {
        self.other_penalty_length_threshold
    }

    /// Sets the other character length threshold.
    ///
    /// # Arguments
    ///
    /// * `value` - An integer of 0 or more.
    ///
    /// # Returns
    ///
    /// Nothing, or a thrown string for any other number, which leaves the
    /// threshold unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_other_penalty_length_threshold(&mut self, value: f64) -> Result<(), JsValue> {
        self.other_penalty_length_threshold = integer_arg("other_penalty_length_threshold", value)?;
        Ok(())
    }

    /// Penalty value for long other-character sequences.
    #[wasm_bindgen(getter)]
    pub fn other_penalty_length_penalty(&self) -> i32 {
        self.other_penalty_length_penalty
    }

    /// Sets the other character penalty value.
    ///
    /// # Arguments
    ///
    /// * `value` - A 32-bit signed integer.
    ///
    /// # Returns
    ///
    /// Nothing, or a thrown string for any other number, which leaves the
    /// penalty unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_other_penalty_length_penalty(&mut self, value: f64) -> Result<(), JsValue> {
        self.other_penalty_length_penalty = integer_arg("other_penalty_length_penalty", value)?;
        Ok(())
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

/// Reads the optional `Penalty` argument of `setMode` and the `Tokenizer`
/// constructor (#1215).
///
/// The argument is taken as a `JsValue` and read through the four getters of
/// `Penalty`: wasm-bindgen destroys the JS object of a class argument taken
/// by value, so the caller could not pass the same `Penalty` again.
///
/// # Arguments
///
/// * `value` - The argument, or `None` when it is left out.
///
/// # Returns
///
/// `None` for an argument that is left out, `null` or `undefined`, the core
/// penalty otherwise, or a thrown string for a value that is not a
/// `Penalty`: one that is not an object, or one whose properties are not
/// integers in the range of the fields.
pub fn penalty_from_js(value: Option<JsValue>) -> Result<Option<LinderaPenalty>, JsValue> {
    let Some(value) = value.filter(|value| !value.is_null() && !value.is_undefined()) else {
        return Ok(None);
    };
    if !value.is_object() {
        return Err(JsValue::from_str("penalty must be a Penalty"));
    }
    let field = |name: &str| -> Result<f64, JsValue> {
        js_sys::Reflect::get(&value, &JsValue::from_str(name))?
            .as_f64()
            .ok_or_else(|| {
                JsValue::from_str(&format!(
                    "penalty must be a Penalty, but its {name} is not a number"
                ))
            })
    };
    Ok(Some(LinderaPenalty {
        kanji_penalty_length_threshold: integer_arg(
            "kanji_penalty_length_threshold",
            field("kanji_penalty_length_threshold")?,
        )?,
        kanji_penalty_length_penalty: integer_arg(
            "kanji_penalty_length_penalty",
            field("kanji_penalty_length_penalty")?,
        )?,
        other_penalty_length_threshold: integer_arg(
            "other_penalty_length_threshold",
            field("other_penalty_length_threshold")?,
        )?,
        other_penalty_length_penalty: integer_arg(
            "other_penalty_length_penalty",
            field("other_penalty_length_penalty")?,
        )?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    /// A `Penalty` is read through its getters and stays usable, and `null`,
    /// `undefined` or a left-out argument mean the default (#1215).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_penalty_from_js_reads_a_penalty() {
        let penalty =
            JsValue::from(JsPenalty::new(Some(3.0), Some(-5000.0), None, Some(0.0)).unwrap());
        let expected = LinderaPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: -5000,
            other_penalty_length_threshold: 7,
            other_penalty_length_penalty: 0,
        };
        assert_eq!(
            penalty_from_js(Some(penalty.clone())).unwrap(),
            Some(expected.clone())
        );
        assert_eq!(penalty_from_js(Some(penalty)).unwrap(), Some(expected));

        assert_eq!(penalty_from_js(None).unwrap(), None);
        assert_eq!(penalty_from_js(Some(JsValue::NULL)).unwrap(), None);
        assert_eq!(penalty_from_js(Some(JsValue::UNDEFINED)).unwrap(), None);
    }

    /// A value that is not a `Penalty` is an error, not the default.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_penalty_from_js_rejects_other_values() {
        let partial = js_sys::Object::new();
        js_sys::Reflect::set(&partial, &"kanji_penalty_length_penalty".into(), &0.into()).unwrap();
        for value in [
            JsValue::from_f64(0.0),
            JsValue::from_str("penalty"),
            JsValue::TRUE,
            partial.into(),
        ] {
            let shown = format!("{value:?}");
            let error = penalty_from_js(Some(value))
                .unwrap_err()
                .as_string()
                .unwrap();
            assert!(error.contains("must be a Penalty"), "{shown}: {error}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_penalty_new_defaults_wasm() {
        let penalty = JsPenalty::new(None, None, None, None).unwrap();

        assert_eq!(penalty.kanji_penalty_length_threshold, 2);
        assert_eq!(penalty.kanji_penalty_length_penalty, 3000);
        assert_eq!(penalty.other_penalty_length_threshold, 7);
        assert_eq!(penalty.other_penalty_length_penalty, 1700);
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_penalty_new_custom_wasm() {
        let penalty = JsPenalty::new(Some(3.0), Some(5000.0), Some(10.0), Some(2000.0)).unwrap();

        assert_eq!(penalty.kanji_penalty_length_threshold, 3);
        assert_eq!(penalty.kanji_penalty_length_penalty, 5000);
        assert_eq!(penalty.other_penalty_length_threshold, 10);
        assert_eq!(penalty.other_penalty_length_penalty, 2000);
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_penalty_new_rejects_numbers_outside_the_range_wasm() {
        let message = |result: Result<JsPenalty, JsValue>| result.unwrap_err().as_string().unwrap();
        assert_eq!(
            message(JsPenalty::new(Some(-1.0), None, None, None)),
            "kanji_penalty_length_threshold must be an integer between 0 and 4294967295, got -1"
        );
        assert_eq!(
            message(JsPenalty::new(Some(4294967301.0), None, None, None)),
            "kanji_penalty_length_threshold must be an integer between 0 and 4294967295, \
             got 4294967301"
        );
        assert_eq!(
            message(JsPenalty::new(None, Some(2147483648.0), None, None)),
            "kanji_penalty_length_penalty must be an integer between -2147483648 and \
             2147483647, got 2147483648"
        );
        assert_eq!(
            message(JsPenalty::new(None, None, Some(f64::NAN), None)),
            "other_penalty_length_threshold must be an integer between 0 and 4294967295, \
             got NaN"
        );
        assert_eq!(
            message(JsPenalty::new(None, None, None, Some(-2147483649.0))),
            "other_penalty_length_penalty must be an integer between -2147483648 and \
             2147483647, got -2147483649"
        );
        assert!(JsPenalty::new(Some(1.5), None, None, None).is_err());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_penalty_setters_reject_numbers_outside_the_range_wasm() {
        let mut penalty = JsPenalty::new(None, None, None, None).unwrap();
        assert!(penalty.set_kanji_penalty_length_threshold(-1.0).is_err());
        assert!(
            penalty
                .set_kanji_penalty_length_penalty(2147483648.0)
                .is_err()
        );
        assert!(
            penalty
                .set_other_penalty_length_threshold(f64::INFINITY)
                .is_err()
        );
        assert!(penalty.set_other_penalty_length_penalty(0.5).is_err());
        // A rejected value leaves the setting unchanged.
        assert_eq!(penalty.kanji_penalty_length_threshold(), 2);
        assert_eq!(penalty.kanji_penalty_length_penalty(), 3000);
        assert_eq!(penalty.other_penalty_length_threshold(), 7);
        assert_eq!(penalty.other_penalty_length_penalty(), 1700);

        penalty
            .set_kanji_penalty_length_threshold(4294967295.0)
            .unwrap();
        penalty
            .set_other_penalty_length_penalty(-2147483648.0)
            .unwrap();
        assert_eq!(penalty.kanji_penalty_length_threshold(), u32::MAX as usize);
        assert_eq!(penalty.other_penalty_length_penalty(), i32::MIN);
    }

    #[test]
    fn test_penalty_new_defaults() {
        let penalty = JsPenalty::new(None, None, None, None).unwrap();

        assert_eq!(penalty.kanji_penalty_length_threshold, 2);
        assert_eq!(penalty.kanji_penalty_length_penalty, 3000);
        assert_eq!(penalty.other_penalty_length_threshold, 7);
        assert_eq!(penalty.other_penalty_length_penalty, 1700);
    }

    #[test]
    fn test_penalty_new_custom() {
        let penalty = JsPenalty::new(Some(3.0), Some(5000.0), Some(10.0), Some(2000.0)).unwrap();

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
        let js_penalty = JsPenalty::new(Some(5.0), Some(4000.0), Some(8.0), Some(1500.0)).unwrap();
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
