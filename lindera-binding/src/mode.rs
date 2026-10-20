//! Segmentation-mode helpers shared by the bindings.
//!
//! Each binding exports a `Mode` class that holds a [`lindera::mode::Mode`]
//! and passes it to [`crate::CoreTokenizerBuilder::set_mode`] and the
//! [`crate::CoreTokenizer`] constructors, as the Rust API takes a `Mode`
//! (#1218). This module holds the conversion from a mode name that the
//! bindings' `Mode.from_str` / `Mode.fromStr` share.

use std::str::FromStr;

use lindera::mode::Mode;

use crate::error::CoreResult;

/// Parses a mode name as [`Mode::from_str`] does.
///
/// # Arguments
///
/// * `name` - `"normal"` or `"decompose"`, case sensitive.
///
/// # Returns
///
/// The mode, with the default penalty for `"decompose"`, or an
/// invalid-argument error for any other name.
pub fn parse_mode(name: &str) -> CoreResult<Mode> {
    Ok(Mode::from_str(name)?)
}

#[cfg(test)]
mod tests {
    use lindera::mode::Penalty;

    use super::*;
    use crate::error::ErrorKind;

    #[test]
    fn parse_mode_reads_the_two_names() {
        assert_eq!(parse_mode("normal").ok(), Some(Mode::Normal));
        assert_eq!(
            parse_mode("decompose").ok(),
            Some(Mode::Decompose(Penalty::default()))
        );
    }

    /// The names are case sensitive, as the core's `FromStr` is; an unknown
    /// name is the invalid-argument error that `set_mode` gave for it in v6.
    #[test]
    fn parse_mode_rejects_other_names() {
        for name in ["Normal", "DECOMPOSE", "", "definitely-not-a-mode"] {
            match parse_mode(name) {
                Ok(mode) => panic!("{name:?} gave {mode:?}"),
                Err(err) => assert_eq!(err.kind(), ErrorKind::InvalidArgument, "{name:?}"),
            }
        }
    }
}
