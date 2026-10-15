//! Checks on argument values that the bindings convert.
//!
//! # Nesting limit
//!
//! `TokenizerBuilder.setSpacePenalty`, `appendCharacterFilter` and
//! `appendTokenFilter` take a host-language value (a JS object, a Python
//! dict, a Ruby hash, a PHP array) and convert it to a `serde_json::Value`
//! by recursing into every container. Without a limit, a value that contains
//! itself, or one nested a few thousand levels deep, overflows the native
//! stack and takes the host process down. Every binding counts the nesting
//! while it converts and stops at [`MAX_ARGUMENT_DEPTH`], so such a value
//! becomes an ordinary, catchable error instead.
//!
//! # Integer range
//!
//! An integer argument sets a value of a narrower type in the core, such as
//! the `i16` default word cost or the `usize` number of N-best results. A
//! binding that receives a wider value (PHP's `int`, a JS number) converts it
//! with [`integer_argument`] or [`integer_argument_from_f64`], which reject a
//! value the target type cannot hold instead of wrapping it, and name the
//! parameter and the accepted range in the message.

use std::fmt;

/// Deepest container nesting a converted argument may have.
///
/// Containers (arrays, objects, dicts, hashes) count; the outermost one is
/// level 1, and a container at level `MAX_ARGUMENT_DEPTH + 1` is rejected.
/// A value that contains itself is infinitely deep, so this limit also stops
/// cycles. Real configuration objects are a few levels deep.
pub const MAX_ARGUMENT_DEPTH: usize = 128;

/// Returns the error message for an argument deeper than
/// [`MAX_ARGUMENT_DEPTH`].
///
/// # Returns
///
/// The message every binding raises, so the error reads the same in each
/// language.
pub fn argument_too_deep_message() -> String {
    format!(
        "argument is nested more than {MAX_ARGUMENT_DEPTH} levels deep \
         (a value that contains itself is always too deep)"
    )
}

/// An integer type that a binding converts an integer argument to.
///
/// The bounds are widened to `i128`, which holds the bounds of every
/// implementing type as well as every `i64`, so a value can be compared
/// with them without loss.
pub trait IntegerArgument: TryFrom<i128> {
    /// The smallest value of the type.
    const MIN: i128;
    /// The largest value of the type.
    const MAX: i128;
}

/// Implements [`IntegerArgument`] for primitive integer types.
macro_rules! impl_integer_argument {
    ($($t:ty),*) => {$(
        impl IntegerArgument for $t {
            /// The type's `MIN`, widened to `i128`.
            const MIN: i128 = <$t>::MIN as i128;
            /// The type's `MAX`, widened to `i128`.
            const MAX: i128 = <$t>::MAX as i128;
        }
    )*};
}

impl_integer_argument!(i16, u16, i32, u32, i64, u64, usize);

/// Converts an integer argument to the type the core stores it as.
///
/// # Arguments
///
/// * `name` - The parameter's name, as the caller writes it in the binding's
///   language (for example `default_word_cost` in PHP).
/// * `value` - The value passed.
///
/// # Returns
///
/// The value as `T`, or the message from [`integer_argument_message`] when
/// `T` cannot hold it.
pub fn integer_argument<T: IntegerArgument>(name: &str, value: i64) -> Result<T, String> {
    T::try_from(i128::from(value)).map_err(|_| integer_argument_message::<T>(name, value))
}

/// Converts a JS number passed as an integer argument to the type the core
/// stores it as.
///
/// A JS number is an `f64`, so besides a value outside `T`'s range this also
/// rejects `NaN`, ±`Infinity` and a fraction, none of which is an integer
/// `T` can hold.
///
/// # Arguments
///
/// * `name` - The parameter's name, as the caller writes it (for example
///   `defaultWordCost` in Node.js).
/// * `value` - The number passed.
///
/// # Returns
///
/// The value as `T`, or the message from [`integer_argument_message`] when it
/// is not an integer in `T`'s range.
pub fn integer_argument_from_f64<T: IntegerArgument>(name: &str, value: f64) -> Result<T, String> {
    let message = || integer_argument_message::<T>(name, JsNumber(value));
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(message());
    }
    // An integral value beyond `i128` saturates, which is still outside every
    // implementing type's range.
    T::try_from(value as i128).map_err(|_| message())
}

/// Returns the error message for an integer argument that `T` cannot hold.
///
/// # Arguments
///
/// * `name` - The parameter's name, as the caller writes it.
/// * `value` - The value passed, as the caller would write it.
///
/// # Returns
///
/// The message every binding raises, such as `default_word_cost must be an
/// integer between -32768 and 32767, got 100000`.
pub fn integer_argument_message<T: IntegerArgument>(
    name: &str,
    value: impl fmt::Display,
) -> String {
    format!(
        "{name} must be an integer between {} and {}, got {value}",
        T::MIN,
        T::MAX
    )
}

/// An `f64` displayed as JavaScript writes a number, for error messages.
struct JsNumber(f64);

impl fmt::Display for JsNumber {
    /// Writes `NaN`, `Infinity` and `-Infinity` by name, an integer below
    /// `1e21` in full, a larger one in exponent form (`1e+300`), and anything
    /// else as Rust's shortest representation (`1.5`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0;
        if value.is_nan() {
            f.write_str("NaN")
        } else if value.is_infinite() {
            f.write_str(if value > 0.0 { "Infinity" } else { "-Infinity" })
        } else if value == 0.0 {
            f.write_str("0")
        } else if value.abs() >= 1e21 {
            let exponent = format!("{value:e}");
            match exponent.split_once('e') {
                Some((mantissa, power)) if !power.starts_with('-') => {
                    write!(f, "{mantissa}e+{power}")
                }
                _ => f.write_str(&exponent),
            }
        } else if value.fract() == 0.0 {
            write!(f, "{value:.0}")
        } else {
            write!(f, "{value}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_too_deep_message_names_the_limit() {
        let message = argument_too_deep_message();
        assert!(message.contains("nested more than 128 levels deep"));
        assert!(message.contains("contains itself"));
    }

    #[test]
    fn test_integer_argument_accepts_the_bounds() {
        assert_eq!(integer_argument::<i16>("x", -32768), Ok(i16::MIN));
        assert_eq!(integer_argument::<i16>("x", 32767), Ok(i16::MAX));
        assert_eq!(integer_argument::<u16>("x", 0), Ok(0));
        assert_eq!(integer_argument::<u16>("x", 65535), Ok(u16::MAX));
        assert_eq!(integer_argument::<i32>("x", -2147483648), Ok(i32::MIN));
        assert_eq!(integer_argument::<i32>("x", 2147483647), Ok(i32::MAX));
        assert_eq!(integer_argument::<u64>("x", i64::MAX), Ok(i64::MAX as u64));
        assert_eq!(integer_argument::<usize>("x", 0), Ok(0));
        assert_eq!(integer_argument::<i64>("x", i64::MIN), Ok(i64::MIN));
    }

    #[test]
    fn test_integer_argument_rejects_values_outside_the_bounds() {
        assert_eq!(
            integer_argument::<i16>("default_word_cost", 100000),
            Err("default_word_cost must be an integer between -32768 and 32767, got 100000".into())
        );
        assert_eq!(
            integer_argument::<i16>("default_word_cost", -32769),
            Err("default_word_cost must be an integer between -32768 and 32767, got -32769".into())
        );
        assert_eq!(
            integer_argument::<u16>("default_left_context_id", -1),
            Err("default_left_context_id must be an integer between 0 and 65535, got -1".into())
        );
        assert_eq!(
            integer_argument::<u16>("default_left_context_id", 65536),
            Err("default_left_context_id must be an integer between 0 and 65535, got 65536".into())
        );
        assert!(integer_argument::<i32>("x", 2147483648).is_err());
        assert!(integer_argument::<i32>("x", -2147483649).is_err());
        assert_eq!(
            integer_argument::<u64>("max_iter", -1),
            Err(format!(
                "max_iter must be an integer between 0 and {}, got -1",
                u64::MAX
            ))
        );
        assert_eq!(
            integer_argument::<usize>("n", -1),
            Err(format!(
                "n must be an integer between 0 and {}, got -1",
                usize::MAX
            ))
        );
    }

    #[test]
    fn test_integer_argument_from_f64_accepts_integral_values_in_range() {
        assert_eq!(
            integer_argument_from_f64::<i16>("x", -32768.0),
            Ok(i16::MIN)
        );
        assert_eq!(integer_argument_from_f64::<i16>("x", 32767.0), Ok(i16::MAX));
        assert_eq!(integer_argument_from_f64::<u16>("x", -0.0), Ok(0));
        assert_eq!(
            integer_argument_from_f64::<u32>("x", 4294967295.0),
            Ok(u32::MAX)
        );
        assert_eq!(
            integer_argument_from_f64::<i64>("x", -9007199254740991.0),
            Ok(-9007199254740991)
        );
        assert_eq!(
            integer_argument_from_f64::<usize>("x", 4294967297.0),
            Ok(4294967297)
        );
    }

    #[test]
    fn test_integer_argument_from_f64_rejects_values_outside_the_bounds() {
        assert_eq!(
            integer_argument_from_f64::<u16>("defaultLeftContextId", 4294967301.0),
            Err(
                "defaultLeftContextId must be an integer between 0 and 65535, got 4294967301"
                    .into()
            )
        );
        assert_eq!(
            integer_argument_from_f64::<u16>("defaultLeftContextId", -1.0),
            Err("defaultLeftContextId must be an integer between 0 and 65535, got -1".into())
        );
        assert!(integer_argument_from_f64::<i16>("x", 32768.0).is_err());
        assert!(integer_argument_from_f64::<i16>("x", -32769.0).is_err());
        assert!(integer_argument_from_f64::<u32>("x", 4294967296.0).is_err());
        assert_eq!(
            integer_argument_from_f64::<i64>("costThreshold", 1e300),
            Err(format!(
                "costThreshold must be an integer between {} and {}, got 1e+300",
                i64::MIN,
                i64::MAX
            ))
        );
        assert!(integer_argument_from_f64::<i64>("x", 9223372036854775808.0).is_err());
        assert_eq!(
            integer_argument_from_f64::<i64>("x", -9223372036854775808.0),
            Ok(i64::MIN)
        );
    }

    #[test]
    fn test_integer_argument_from_f64_rejects_values_that_are_not_integers() {
        assert_eq!(
            integer_argument_from_f64::<i16>("defaultWordCost", 1.5),
            Err("defaultWordCost must be an integer between -32768 and 32767, got 1.5".into())
        );
        assert_eq!(
            integer_argument_from_f64::<i16>("defaultWordCost", f64::NAN),
            Err("defaultWordCost must be an integer between -32768 and 32767, got NaN".into())
        );
        assert_eq!(
            integer_argument_from_f64::<i16>("defaultWordCost", f64::INFINITY),
            Err("defaultWordCost must be an integer between -32768 and 32767, got Infinity".into())
        );
        assert_eq!(
            integer_argument_from_f64::<i16>("defaultWordCost", f64::NEG_INFINITY),
            Err(
                "defaultWordCost must be an integer between -32768 and 32767, got -Infinity".into()
            )
        );
        assert!(integer_argument_from_f64::<usize>("n", 0.5).is_err());
    }

    #[test]
    fn test_js_number_display() {
        assert_eq!(JsNumber(100000.0).to_string(), "100000");
        assert_eq!(JsNumber(-1.0).to_string(), "-1");
        assert_eq!(JsNumber(-0.0).to_string(), "0");
        assert_eq!(JsNumber(1.5).to_string(), "1.5");
        assert_eq!(JsNumber(1e300).to_string(), "1e+300");
        assert_eq!(JsNumber(-1e300).to_string(), "-1e+300");
        assert_eq!(JsNumber(f64::NAN).to_string(), "NaN");
    }
}
