//! Tokenization modes and penalty configurations.
//!
//! This module defines the different tokenization modes available and their
//! penalty configurations for controlling segmentation behavior.

use magnus::prelude::*;
use magnus::{Error, Ruby, function, method};

use lindera::mode::{Mode as LinderaMode, Penalty as LinderaPenalty};

/// Tokenization mode.
///
/// Determines how text is segmented into tokens.
#[magnus::wrap(class = "Lindera::Mode", free_immediately, size)]
#[derive(Debug, Clone, Copy)]
pub struct RbMode {
    /// Internal mode variant.
    inner: RbModeKind,
}

/// Internal enum for mode kind.
#[derive(Debug, Clone, Copy)]
enum RbModeKind {
    /// Standard tokenization based on dictionary cost.
    Normal,
    /// Decompose compound words using penalty-based segmentation.
    Decompose,
}

impl RbMode {
    /// Creates a new `RbMode` from a mode string.
    ///
    /// # Arguments
    ///
    /// * `mode_str` - Mode string ("normal" or "decompose"). Defaults to "normal" if None.
    ///
    /// # Returns
    ///
    /// A new `RbMode` instance.
    fn new(mode_str: Option<String>) -> Result<Self, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not initialized");
        let kind = match mode_str.as_deref() {
            Some("decompose") | Some("Decompose") => RbModeKind::Decompose,
            Some("normal") | Some("Normal") | None => RbModeKind::Normal,
            Some(s) => {
                return Err(Error::new(
                    ruby.exception_arg_error(),
                    format!("Invalid mode: {s}. Must be 'normal' or 'decompose'"),
                ));
            }
        };
        Ok(Self { inner: kind })
    }

    /// Returns the string representation of the mode.
    ///
    /// # Returns
    ///
    /// A string slice representing the mode.
    fn to_s(&self) -> &str {
        match self.inner {
            RbModeKind::Normal => "normal",
            RbModeKind::Decompose => "decompose",
        }
    }

    /// Returns the inspect representation of the mode.
    ///
    /// # Returns
    ///
    /// A string with the mode inspect representation.
    fn inspect(&self) -> String {
        format!("#<Lindera::Mode: {}>", self.to_s())
    }

    /// Returns the name of the mode.
    ///
    /// # Returns
    ///
    /// A string slice with the mode name.
    fn name(&self) -> &str {
        self.to_s()
    }

    /// Returns true if the mode is normal.
    ///
    /// # Returns
    ///
    /// A boolean indicating if this is normal mode.
    fn is_normal(&self) -> bool {
        matches!(self.inner, RbModeKind::Normal)
    }

    /// Returns true if the mode is decompose.
    ///
    /// # Returns
    ///
    /// A boolean indicating if this is decompose mode.
    fn is_decompose(&self) -> bool {
        matches!(self.inner, RbModeKind::Decompose)
    }
}

impl From<RbMode> for LinderaMode {
    fn from(mode: RbMode) -> Self {
        match mode.inner {
            RbModeKind::Normal => LinderaMode::Normal,
            RbModeKind::Decompose => LinderaMode::Decompose(LinderaPenalty::default()),
        }
    }
}

impl From<LinderaMode> for RbMode {
    fn from(mode: LinderaMode) -> Self {
        let kind = match mode {
            LinderaMode::Normal => RbModeKind::Normal,
            LinderaMode::Decompose(_) => RbModeKind::Decompose,
        };
        RbMode { inner: kind }
    }
}

/// Penalty configuration for decompose mode.
///
/// Controls how aggressively compound words are decomposed based on
/// character type and length thresholds.
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
    mode_class.define_singleton_method("new", function!(RbMode::new, 1))?;
    mode_class.define_method("to_s", method!(RbMode::to_s, 0))?;
    mode_class.define_method("inspect", method!(RbMode::inspect, 0))?;
    mode_class.define_method("name", method!(RbMode::name, 0))?;
    mode_class.define_method("normal?", method!(RbMode::is_normal, 0))?;
    mode_class.define_method("decompose?", method!(RbMode::is_decompose, 0))?;

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
