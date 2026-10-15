//! Conversion of integer arguments from JS values.
//!
//! wasm-bindgen's generated JS converts a number passed to a `usize`, `u32`
//! or `i32` parameter with `>>> 0`, `>> 0` or ToInt32, which wrap modulo 2^32
//! and turn `NaN`, `Infinity` and fractions into other integers before Rust
//! sees them, and a BigInt passed to an `i64` parameter modulo 2^64
//! (lindera/lindera#1209). Integer parameters are therefore declared as `f64`,
//! which holds every JS number exactly, or as a `JsValue` for a BigInt, and
//! converted here. The checks themselves are
//! [`lindera_binding::argument`]'s, so the messages read as in the other
//! bindings.

use lindera_binding::argument::{
    IntegerArgument, integer_argument_from_f64, integer_argument_message,
};
use wasm_bindgen::prelude::*;

/// Converts a JS number passed as an integer argument to the type the core
/// stores it as.
///
/// # Arguments
///
/// * `name` - The parameter's name as JS callers write it.
/// * `value` - The number passed.
///
/// # Returns
///
/// The value as `T`, or the error message, thrown as a string like the other
/// errors of this crate, when the number is not an integer `T` can hold.
pub fn integer_arg<T: IntegerArgument>(name: &str, value: f64) -> Result<T, JsValue> {
    integer_argument_from_f64(name, value).map_err(|message| JsValue::from_str(&message))
}

/// Converts an optional JS number with [`integer_arg`].
///
/// # Arguments
///
/// * `name` - The parameter's name as JS callers write it.
/// * `value` - The number passed, or `None` for `null` or `undefined`.
///
/// # Returns
///
/// `None` for `None`, the value as `T` otherwise, or the error of
/// [`integer_arg`].
pub fn optional_integer_arg<T: IntegerArgument>(
    name: &str,
    value: Option<f64>,
) -> Result<Option<T>, JsValue> {
    value.map(|value| integer_arg(name, value)).transpose()
}

/// Converts an optional BigInt passed as an `i64` argument.
///
/// # Arguments
///
/// * `name` - The parameter's name as JS callers write it.
/// * `value` - The value passed; `null` and `undefined` mean none.
///
/// # Returns
///
/// `None` for `null` or `undefined`, the value as an `i64` for a BigInt in
/// its range, or a thrown string for any other value.
pub fn optional_i64_from_bigint(
    name: &str,
    value: Option<JsValue>,
) -> Result<Option<i64>, JsValue> {
    let Some(value) = value.filter(|value| !value.is_null() && !value.is_undefined()) else {
        return Ok(None);
    };
    let Some(bigint) = value.dyn_ref::<js_sys::BigInt>() else {
        return Err(JsValue::from_str(&format!("{name} must be a BigInt")));
    };
    i64::try_from(value.clone()).map(Some).map_err(|_| {
        let shown = bigint.to_string(10).map(String::from).unwrap_or_default();
        JsValue::from_str(&integer_argument_message::<i64>(name, shown))
    })
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_integer_arg_rejects_numbers_that_do_not_fit() {
        assert_eq!(
            integer_arg::<usize>("n", 4294967295.0).unwrap(),
            u32::MAX as usize
        );
        let error = integer_arg::<usize>("n", 4294967296.0).unwrap_err();
        assert_eq!(
            error.as_string().unwrap(),
            "n must be an integer between 0 and 4294967295, got 4294967296"
        );
        assert!(integer_arg::<usize>("n", -1.0).is_err());
        assert!(integer_arg::<usize>("n", 1.5).is_err());
        assert!(integer_arg::<usize>("n", f64::NAN).is_err());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_optional_i64_from_bigint() {
        let bigint =
            |text: &str| JsValue::from(js_sys::BigInt::new(&JsValue::from_str(text)).unwrap());
        assert_eq!(optional_i64_from_bigint("x", None).unwrap(), None);
        assert_eq!(
            optional_i64_from_bigint("x", Some(JsValue::NULL)).unwrap(),
            None
        );
        assert_eq!(
            optional_i64_from_bigint("x", Some(bigint("-9223372036854775808"))).unwrap(),
            Some(i64::MIN)
        );
        assert_eq!(
            optional_i64_from_bigint("costThreshold", Some(bigint("18446744073709551616")))
                .unwrap_err()
                .as_string()
                .unwrap(),
            "costThreshold must be an integer between -9223372036854775808 and \
             9223372036854775807, got 18446744073709551616"
        );
        assert!(optional_i64_from_bigint("x", Some(bigint("-9223372036854775809"))).is_err());
        assert_eq!(
            optional_i64_from_bigint("x", Some(JsValue::from_f64(5.0)))
                .unwrap_err()
                .as_string()
                .unwrap(),
            "x must be a BigInt"
        );
    }
}
