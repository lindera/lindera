//! Tokenization modes and penalty configurations for PHP.
//!
//! This module defines the different tokenization modes available and their
//! penalty configurations for controlling segmentation behavior.

use ext_php_rs::prelude::*;

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

use crate::convert::optional_int;
use crate::error::lindera_value_err;

/// Tokenization mode: normal, or decompose with a `Lindera\Penalty`.
///
/// The PHP counterpart of `lindera::mode::Mode`, which `TokenizerBuilder::setMode()`
/// and the `Tokenizer` constructor take. It has no public constructor;
/// `Mode::normal()`, `Mode::decompose()` and `Mode::fromStr()` create one. A
/// mode cannot be changed, and `==` compares its name and penalty.
#[php_class]
#[php(name = "Lindera\\Mode")]
#[derive(Debug, Clone, PartialEq)]
pub struct PhpMode {
    /// The mode, with its penalty for decompose mode.
    inner: LinderaMode,
}

#[php_impl]
impl PhpMode {
    /// Creates normal mode.
    ///
    /// # Returns
    ///
    /// Normal mode.
    pub fn normal() -> Self {
        Self {
            inner: LinderaMode::Normal,
        }
    }

    /// Creates decompose mode.
    ///
    /// # Arguments
    ///
    /// * `penalty` - The `Lindera\Penalty`. `null` (the default) uses the
    ///   default penalty. A value of another type throws instead of
    ///   selecting the default.
    ///
    /// # Returns
    ///
    /// Decompose mode with the penalty.
    pub fn decompose(penalty: Option<&PhpPenalty>) -> Self {
        Self::decompose_with(penalty.map(|penalty| penalty.clone().into()))
    }

    /// Creates a mode from its name, as the Rust `Mode` parses one.
    ///
    /// # Arguments
    ///
    /// * `name` - `"normal"` or `"decompose"`, case sensitive.
    ///
    /// # Returns
    ///
    /// The mode, with the default penalty for `"decompose"`, or a
    /// `ValueError` for any other name, as `setMode()` threw for it in v6.
    #[php(name = "fromStr")]
    pub fn parse(name: String) -> PhpResult<Self> {
        let inner = lindera_binding::parse_mode(&name).map_err(lindera_value_err)?;
        Ok(Self { inner })
    }

    /// Returns the mode name.
    ///
    /// # Returns
    ///
    /// `"normal"` or `"decompose"`.
    #[php(getter, change_case = "snake_case")]
    pub fn name(&self) -> String {
        self.inner.as_str().to_string()
    }

    /// Returns the penalty of decompose mode.
    ///
    /// # Returns
    ///
    /// A copy of the penalty, or `null` for normal mode.
    #[php(getter, change_case = "snake_case")]
    pub fn penalty(&self) -> Option<PhpPenalty> {
        match &self.inner {
            LinderaMode::Normal => None,
            LinderaMode::Decompose(penalty) => Some(penalty.clone().into()),
        }
    }

    /// Returns whether this is the normal mode.
    ///
    /// # Returns
    ///
    /// True if the mode is normal.
    pub fn is_normal(&self) -> bool {
        matches!(self.inner, LinderaMode::Normal)
    }

    /// Returns whether this is the decompose mode.
    ///
    /// # Returns
    ///
    /// True if the mode is decompose.
    pub fn is_decompose(&self) -> bool {
        matches!(self.inner, LinderaMode::Decompose(_))
    }

    /// Returns a string representation of the mode.
    ///
    /// # Returns
    ///
    /// The mode name.
    pub fn __to_string(&self) -> String {
        self.name()
    }
}

impl PhpMode {
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

/// Penalty configuration for decompose mode.
///
/// Controls how aggressively compound words are decomposed based on
/// character type and length thresholds. Pass it to `Mode::decompose()`.
#[php_class]
#[php(name = "Lindera\\Penalty")]
#[derive(Clone)]
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
    /// * `kanji_penalty_length_threshold` - Kanji length threshold, 0 or more
    ///   (default: 2).
    /// * `kanji_penalty_length_penalty` - Kanji penalty value, a 32-bit
    ///   signed integer (default: 3000).
    /// * `other_penalty_length_threshold` - Other character length threshold,
    ///   0 or more (default: 7).
    /// * `other_penalty_length_penalty` - Other character penalty value, a
    ///   32-bit signed integer (default: 1700).
    ///
    /// A value of another type than `int` or `null` throws instead of
    /// selecting the default.
    ///
    /// # Returns
    ///
    /// A new Penalty instance, or a `ValueError` for a value outside its
    /// range.
    pub fn __construct(
        kanji_penalty_length_threshold: Option<i64>,
        kanji_penalty_length_penalty: Option<i64>,
        other_penalty_length_threshold: Option<i64>,
        other_penalty_length_penalty: Option<i64>,
    ) -> PhpResult<Self> {
        Self::from_args(
            kanji_penalty_length_threshold,
            kanji_penalty_length_penalty,
            other_penalty_length_threshold,
            other_penalty_length_penalty,
        )
        .map_err(lindera_value_err)
    }

    /// Returns the kanji penalty length threshold.
    ///
    /// # Returns
    ///
    /// The threshold value.
    #[php(getter, change_case = "snake_case")]
    pub fn kanji_penalty_length_threshold(&self) -> i64 {
        self.kanji_penalty_length_threshold as i64
    }

    /// Returns the kanji penalty value.
    ///
    /// # Returns
    ///
    /// The penalty value.
    #[php(getter, change_case = "snake_case")]
    pub fn kanji_penalty_length_penalty(&self) -> i64 {
        self.kanji_penalty_length_penalty as i64
    }

    /// Returns the other character penalty length threshold.
    ///
    /// # Returns
    ///
    /// The threshold value.
    #[php(getter, change_case = "snake_case")]
    pub fn other_penalty_length_threshold(&self) -> i64 {
        self.other_penalty_length_threshold as i64
    }

    /// Returns the other character penalty value.
    ///
    /// # Returns
    ///
    /// The penalty value.
    #[php(getter, change_case = "snake_case")]
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

impl PhpPenalty {
    /// Builds a penalty from the constructor's arguments.
    ///
    /// The checks live here, apart from `__construct`, so that Rust tests can
    /// call them: turning the message into a PHP exception links symbols
    /// that only the PHP runtime provides.
    ///
    /// # Arguments
    ///
    /// The arguments of `__construct`, with `None` for `null` or an omitted
    /// argument, which selects the core's default.
    ///
    /// # Returns
    ///
    /// The penalty, or the message for a value outside its range.
    fn from_args(
        kanji_penalty_length_threshold: Option<i64>,
        kanji_penalty_length_penalty: Option<i64>,
        other_penalty_length_threshold: Option<i64>,
        other_penalty_length_penalty: Option<i64>,
    ) -> Result<Self, String> {
        let default = LinderaPenalty::default();
        Ok(Self {
            kanji_penalty_length_threshold: optional_int(
                "kanji_penalty_length_threshold",
                kanji_penalty_length_threshold,
            )?
            .unwrap_or(default.kanji_penalty_length_threshold),
            kanji_penalty_length_penalty: optional_int(
                "kanji_penalty_length_penalty",
                kanji_penalty_length_penalty,
            )?
            .unwrap_or(default.kanji_penalty_length_penalty),
            other_penalty_length_threshold: optional_int(
                "other_penalty_length_threshold",
                other_penalty_length_threshold,
            )?
            .unwrap_or(default.other_penalty_length_threshold),
            other_penalty_length_penalty: optional_int(
                "other_penalty_length_penalty",
                other_penalty_length_penalty,
            )?
            .unwrap_or(default.other_penalty_length_penalty),
        })
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

    #[test]
    fn test_phpmode_factories() {
        assert_eq!(PhpMode::normal().as_lindera(), &LinderaMode::Normal);
        assert_eq!(
            PhpMode::decompose_with(None).as_lindera(),
            &LinderaMode::Decompose(LinderaPenalty::default())
        );
        let penalty = LinderaPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 0,
            other_penalty_length_threshold: 4,
            other_penalty_length_penalty: 3000,
        };
        let mode = PhpMode::decompose_with(Some(penalty.clone()));
        assert_eq!(mode.as_lindera(), &LinderaMode::Decompose(penalty));
        let Some(back) = mode.penalty() else {
            panic!("decompose mode has no penalty");
        };
        assert_eq!(back.kanji_penalty_length_threshold, 3);
        assert_eq!(back.other_penalty_length_penalty, 3000);
        assert!(PhpMode::normal().penalty().is_none());
    }

    #[test]
    fn test_phpmode_names_and_kinds() {
        let normal = PhpMode::normal();
        let decompose = PhpMode::decompose_with(None);
        assert_eq!(normal.name(), "normal");
        assert_eq!(decompose.__to_string(), "decompose");
        assert!(normal.is_normal() && !normal.is_decompose());
        assert!(decompose.is_decompose() && !decompose.is_normal());
    }

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
        let php_penalty = PhpPenalty::from_args(None, None, None, None).unwrap();
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

    #[test]
    fn test_phppenalty_from_args_rejects_values_outside_the_range() {
        let error = |result: Result<PhpPenalty, String>| result.err();
        assert_eq!(
            error(PhpPenalty::from_args(Some(-1), None, None, None)),
            Some(format!(
                "kanji_penalty_length_threshold must be an integer between 0 and {}, got -1",
                usize::MAX
            ))
        );
        assert_eq!(
            error(PhpPenalty::from_args(None, Some(2147483648), None, None)),
            Some(
                "kanji_penalty_length_penalty must be an integer between -2147483648 and \
                 2147483647, got 2147483648"
                    .into()
            )
        );
        assert_eq!(
            error(PhpPenalty::from_args(None, None, Some(-1), None)),
            Some(format!(
                "other_penalty_length_threshold must be an integer between 0 and {}, got -1",
                usize::MAX
            ))
        );
        assert_eq!(
            error(PhpPenalty::from_args(None, None, None, Some(-2147483649))),
            Some(
                "other_penalty_length_penalty must be an integer between -2147483648 and \
                 2147483647, got -2147483649"
                    .into()
            )
        );
        let penalty = PhpPenalty::from_args(
            Some(0),
            Some(i32::MIN.into()),
            Some(0),
            Some(i32::MAX.into()),
        )
        .unwrap();
        assert_eq!(penalty.kanji_penalty_length_penalty, i32::MIN);
        assert_eq!(penalty.other_penalty_length_penalty, i32::MAX);
    }
}
