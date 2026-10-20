//! Tokenization modes and penalty configurations.
//!
//! This module defines the different tokenization modes available and their
//! penalty configurations for controlling segmentation behavior. A
//! `Lindera::Mode` holds the penalty of decompose mode, as the Rust
//! `lindera::mode::Mode` does, and is what `TokenizerBuilder#set_mode` and
//! `Tokenizer.new` take (#1218).

use magnus::prelude::*;
use magnus::scan_args::scan_args;
use magnus::{Error, Ruby, TryConvert, Value, function, method};

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

use crate::error::to_magnus_error;

/// Tokenization mode: normal, or decompose with a `Lindera::Penalty`.
///
/// The Ruby counterpart of `lindera::mode::Mode`. It has no `new`;
/// `Mode.normal`, `Mode.decompose` and `Mode.from_str` create one. A mode
/// cannot be changed, and `==` compares the name and the penalty. Like the
/// Rust `Mode`, it does not define `eql?` and `hash`.
#[magnus::wrap(class = "Lindera::Mode", free_immediately, size)]
#[derive(Debug, Clone, PartialEq)]
pub struct RbMode {
    /// The mode, with its penalty for decompose mode.
    inner: LinderaMode,
}

impl RbMode {
    /// Returns the wrapped `lindera::mode::Mode`.
    pub fn as_lindera(&self) -> &LinderaMode {
        &self.inner
    }

    /// Creates normal mode.
    ///
    /// Ruby signature: `Mode.normal`.
    ///
    /// # Returns
    ///
    /// Normal mode.
    fn normal() -> Self {
        Self {
            inner: LinderaMode::Normal,
        }
    }

    /// Creates decompose mode.
    ///
    /// Ruby signature: `Mode.decompose(penalty = nil)`.
    ///
    /// # Arguments
    ///
    /// * `args` - The Ruby arguments:
    ///   * `penalty` - The `Lindera::Penalty`. `nil` or omitted uses the
    ///     default penalty.
    ///
    /// # Returns
    ///
    /// Decompose mode with the penalty, an `ArgumentError` for more than one
    /// argument, or a `TypeError` for a penalty that is not a
    /// `Lindera::Penalty`.
    fn decompose(args: &[Value]) -> Result<Self, Error> {
        let args = scan_args::<(), (Option<Option<&RbPenalty>>,), (), (), (), ()>(args)?;
        let (penalty,) = args.optional;
        Ok(Self::decompose_with(
            penalty.flatten().map(|penalty| (*penalty).into()),
        ))
    }

    /// Creates decompose mode with `penalty`, or with the default penalty for
    /// `None`.
    fn decompose_with(penalty: Option<LinderaPenalty>) -> Self {
        Self {
            inner: LinderaMode::Decompose(penalty.unwrap_or_default()),
        }
    }

    /// Creates a mode from its name, as the Rust `Mode` parses one.
    ///
    /// Ruby signature: `Mode.from_str(name)`.
    ///
    /// # Arguments
    ///
    /// * `ruby` - Ruby runtime handle.
    /// * `name` - `"normal"` or `"decompose"`, case sensitive.
    ///
    /// # Returns
    ///
    /// The mode, with the default penalty for `"decompose"`, or a
    /// `RuntimeError` for any other name, as `set_mode` raised for it in v6.
    fn parse(ruby: &Ruby, name: String) -> Result<Self, Error> {
        let inner = lindera_binding::parse_mode(&name)
            .map_err(|err| to_magnus_error(ruby, err.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns the name of the mode: `"normal"` or `"decompose"`.
    fn name(&self) -> &str {
        self.inner.as_str()
    }

    /// Returns the name of the mode, as `name` does.
    #[allow(clippy::wrong_self_convention)]
    fn to_s(&self) -> &str {
        self.name()
    }

    /// Returns the inspect representation of the mode, with the penalty of
    /// decompose mode.
    fn inspect(&self) -> String {
        match &self.inner {
            LinderaMode::Normal => "#<Lindera::Mode: normal>".to_string(),
            LinderaMode::Decompose(penalty) => format!(
                "#<Lindera::Mode: decompose {}>",
                RbPenalty::from(penalty.clone()).to_s()
            ),
        }
    }

    /// Returns the penalty of decompose mode, as a copy, or `nil` for normal
    /// mode.
    fn penalty(&self) -> Option<RbPenalty> {
        match &self.inner {
            LinderaMode::Normal => None,
            LinderaMode::Decompose(penalty) => Some(penalty.clone().into()),
        }
    }

    /// Returns true if the mode is normal.
    fn is_normal(&self) -> bool {
        matches!(self.inner, LinderaMode::Normal)
    }

    /// Returns true if the mode is decompose.
    fn is_decompose(&self) -> bool {
        matches!(self.inner, LinderaMode::Decompose(_))
    }

    /// Ruby's `==`: whether `other` is a `Lindera::Mode` with the same name
    /// and penalty.
    ///
    /// # Arguments
    ///
    /// * `other` - Any Ruby value.
    ///
    /// # Returns
    ///
    /// `true` for an equal mode, `false` otherwise, other types included.
    fn is_equal(&self, other: Value) -> bool {
        match <&RbMode>::try_convert(other) {
            Ok(other) => self == other,
            Err(_) => false,
        }
    }
}

impl From<LinderaMode> for RbMode {
    fn from(inner: LinderaMode) -> Self {
        Self { inner }
    }
}

/// Penalty configuration for decompose mode.
///
/// Controls how aggressively compound words are decomposed based on
/// character type and length thresholds. Pass it to `Mode.decompose`.
#[magnus::wrap(class = "Lindera::Penalty", free_immediately, size)]
#[derive(Debug, Clone, Copy)]
pub struct RbPenalty {
    /// Length threshold for kanji penalty.
    kanji_penalty_length_threshold: usize,
    /// Penalty value for kanji.
    kanji_penalty_length_penalty: i32,
    /// Length threshold for other character penalty.
    other_penalty_length_threshold: usize,
    /// Penalty value for other characters.
    other_penalty_length_penalty: i32,
}

impl RbPenalty {
    /// Creates a new `RbPenalty` with optional parameters.
    ///
    /// # Arguments
    ///
    /// * `kanji_threshold` - Kanji penalty length threshold (default: 2).
    /// * `kanji_penalty` - Kanji penalty value (default: 3000).
    /// * `other_threshold` - Other penalty length threshold (default: 7).
    /// * `other_penalty` - Other penalty value (default: 1700).
    ///
    /// # Returns
    ///
    /// A new `RbPenalty` instance.
    fn new(
        kanji_threshold: Option<usize>,
        kanji_penalty: Option<i32>,
        other_threshold: Option<usize>,
        other_penalty: Option<i32>,
    ) -> Self {
        Self {
            kanji_penalty_length_threshold: kanji_threshold.unwrap_or(2),
            kanji_penalty_length_penalty: kanji_penalty.unwrap_or(3000),
            other_penalty_length_threshold: other_threshold.unwrap_or(7),
            other_penalty_length_penalty: other_penalty.unwrap_or(1700),
        }
    }

    /// Returns the kanji penalty length threshold.
    fn kanji_penalty_length_threshold(&self) -> usize {
        self.kanji_penalty_length_threshold
    }

    /// Returns the kanji penalty length penalty.
    fn kanji_penalty_length_penalty(&self) -> i32 {
        self.kanji_penalty_length_penalty
    }

    /// Returns the other penalty length threshold.
    fn other_penalty_length_threshold(&self) -> usize {
        self.other_penalty_length_threshold
    }

    /// Returns the other penalty length penalty.
    fn other_penalty_length_penalty(&self) -> i32 {
        self.other_penalty_length_penalty
    }

    /// Returns a string representation of the penalty configuration.
    #[allow(clippy::wrong_self_convention)]
    fn to_s(&self) -> String {
        format!(
            "Penalty(kanji_threshold={}, kanji_penalty={}, other_threshold={}, other_penalty={})",
            self.kanji_penalty_length_threshold,
            self.kanji_penalty_length_penalty,
            self.other_penalty_length_threshold,
            self.other_penalty_length_penalty
        )
    }

    /// Returns the inspect representation of the penalty.
    fn inspect(&self) -> String {
        format!("#<Lindera::Penalty: {}>", self.to_s())
    }
}

impl From<RbPenalty> for LinderaPenalty {
    fn from(penalty: RbPenalty) -> Self {
        LinderaPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

impl From<LinderaPenalty> for RbPenalty {
    fn from(penalty: LinderaPenalty) -> Self {
        RbPenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

/// Defines Mode and Penalty classes in the given Ruby module.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `module` - Parent Ruby module.
///
/// # Returns
///
/// `Ok(())` on success, or a Magnus `Error` on failure.
pub fn define(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    let mode_class = module.define_class("Mode", ruby.class_object())?;
    // magnus undefines the allocator only when the class first wraps a
    // value, so until then `Mode.new` would return an empty object.
    mode_class.undef_default_alloc_func();
    mode_class.define_singleton_method("normal", function!(RbMode::normal, 0))?;
    mode_class.define_singleton_method("decompose", function!(RbMode::decompose, -1))?;
    mode_class.define_singleton_method("from_str", function!(RbMode::parse, 1))?;
    mode_class.define_method("to_s", method!(RbMode::to_s, 0))?;
    mode_class.define_method("inspect", method!(RbMode::inspect, 0))?;
    mode_class.define_method("name", method!(RbMode::name, 0))?;
    mode_class.define_method("penalty", method!(RbMode::penalty, 0))?;
    mode_class.define_method("normal?", method!(RbMode::is_normal, 0))?;
    mode_class.define_method("decompose?", method!(RbMode::is_decompose, 0))?;
    mode_class.define_method("==", method!(RbMode::is_equal, 1))?;

    let penalty_class = module.define_class("Penalty", ruby.class_object())?;
    penalty_class.define_singleton_method("new", function!(RbPenalty::new, 4))?;
    penalty_class.define_method(
        "kanji_penalty_length_threshold",
        method!(RbPenalty::kanji_penalty_length_threshold, 0),
    )?;
    penalty_class.define_method(
        "kanji_penalty_length_penalty",
        method!(RbPenalty::kanji_penalty_length_penalty, 0),
    )?;
    penalty_class.define_method(
        "other_penalty_length_threshold",
        method!(RbPenalty::other_penalty_length_threshold, 0),
    )?;
    penalty_class.define_method(
        "other_penalty_length_penalty",
        method!(RbPenalty::other_penalty_length_penalty, 0),
    )?;
    penalty_class.define_method("to_s", method!(RbPenalty::to_s, 0))?;
    penalty_class.define_method("inspect", method!(RbPenalty::inspect, 0))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rb_mode_factories() {
        assert_eq!(RbMode::normal().as_lindera(), &LinderaMode::Normal);
        assert_eq!(
            RbMode::decompose_with(None).as_lindera(),
            &LinderaMode::Decompose(LinderaPenalty::default())
        );
        let penalty = LinderaPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 0,
            other_penalty_length_threshold: 4,
            other_penalty_length_penalty: 3000,
        };
        let mode = RbMode::decompose_with(Some(penalty.clone()));
        assert_eq!(mode.as_lindera(), &LinderaMode::Decompose(penalty));
        let Some(back) = mode.penalty() else {
            panic!("decompose mode has no penalty");
        };
        assert_eq!(back.kanji_penalty_length_threshold, 3);
        assert_eq!(back.other_penalty_length_penalty, 3000);
        assert!(RbMode::normal().penalty().is_none());
    }

    #[test]
    fn test_rb_mode_names_and_kinds() {
        let normal = RbMode::normal();
        let decompose = RbMode::decompose_with(None);
        assert_eq!(normal.name(), "normal");
        assert_eq!(decompose.to_s(), "decompose");
        assert!(normal.is_normal() && !normal.is_decompose());
        assert!(decompose.is_decompose() && !decompose.is_normal());
        assert_eq!(normal.inspect(), "#<Lindera::Mode: normal>");
        assert!(
            decompose
                .inspect()
                .starts_with("#<Lindera::Mode: decompose Penalty(")
        );
    }

    /// Two modes are equal when their names and penalties are, as the Rust
    /// `Mode` compares.
    #[test]
    fn test_rb_mode_equality_compares_the_penalty() {
        let zero = LinderaPenalty {
            kanji_penalty_length_penalty: 0,
            other_penalty_length_penalty: 0,
            ..LinderaPenalty::default()
        };
        assert_eq!(RbMode::decompose_with(None), RbMode::decompose_with(None));
        assert_ne!(
            RbMode::decompose_with(None),
            RbMode::decompose_with(Some(zero))
        );
        assert_ne!(RbMode::normal(), RbMode::decompose_with(None));
    }

    #[test]
    fn test_rb_penalty_to_lindera_penalty() {
        let rb_penalty = RbPenalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 5000,
            other_penalty_length_threshold: 10,
            other_penalty_length_penalty: 2500,
        };
        let lindera_penalty: LinderaPenalty = rb_penalty.into();
        assert_eq!(lindera_penalty.kanji_penalty_length_threshold, 3);
        assert_eq!(lindera_penalty.kanji_penalty_length_penalty, 5000);
        assert_eq!(lindera_penalty.other_penalty_length_threshold, 10);
        assert_eq!(lindera_penalty.other_penalty_length_penalty, 2500);
    }

    #[test]
    fn test_lindera_penalty_to_rb_penalty() {
        let lindera_penalty = LinderaPenalty {
            kanji_penalty_length_threshold: 4,
            kanji_penalty_length_penalty: 6000,
            other_penalty_length_threshold: 8,
            other_penalty_length_penalty: 1500,
        };
        let rb_penalty: RbPenalty = lindera_penalty.into();
        assert_eq!(rb_penalty.kanji_penalty_length_threshold, 4);
        assert_eq!(rb_penalty.kanji_penalty_length_penalty, 6000);
        assert_eq!(rb_penalty.other_penalty_length_threshold, 8);
        assert_eq!(rb_penalty.other_penalty_length_penalty, 1500);
    }

    #[test]
    fn test_rb_penalty_default_values_roundtrip() {
        let default_lindera = LinderaPenalty::default();
        let rb_penalty: RbPenalty = default_lindera.clone().into();
        let roundtripped: LinderaPenalty = rb_penalty.into();
        assert_eq!(
            roundtripped.kanji_penalty_length_threshold,
            default_lindera.kanji_penalty_length_threshold
        );
        assert_eq!(
            roundtripped.kanji_penalty_length_penalty,
            default_lindera.kanji_penalty_length_penalty
        );
        assert_eq!(
            roundtripped.other_penalty_length_threshold,
            default_lindera.other_penalty_length_threshold
        );
        assert_eq!(
            roundtripped.other_penalty_length_penalty,
            default_lindera.other_penalty_length_penalty
        );
    }
}
