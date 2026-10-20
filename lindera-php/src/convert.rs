//! Utility functions for PHP-Rust data conversion.
//!
//! This module provides helper functions for converting between PHP Zval objects
//! and Rust data structures, particularly for working with JSON-like data, the
//! argument types [`Nullable`] and [`Float`], which check a PHP argument's
//! type as PHP does, and [`int_arg`], which checks an `int` argument's range.

use ext_php_rs::convert::FromZval;
use ext_php_rs::flags::DataType;
use ext_php_rs::prelude::*;
use ext_php_rs::types::array::ArrayKey;
use ext_php_rs::types::{ZendHashTable, Zval};
use lindera_binding::argument::{
    IntegerArgument, MAX_ARGUMENT_DEPTH, argument_too_deep_message, integer_argument,
};
use serde_json::{Map, Value};

use crate::error::lindera_value_err;

/// Why a PHP value could not be converted to JSON.
#[derive(Debug)]
pub enum ConvertError {
    /// The value, or something inside it, has no JSON form: an object, a
    /// reference, a non-finite float, and so on.
    Unsupported(&'static str),
    /// The value nests arrays more than [`MAX_ARGUMENT_DEPTH`] levels deep.
    TooDeep,
}

impl From<ConvertError> for PhpException {
    /// Throws an unsupported value as a plain `Exception`, as the converter
    /// always has, and a value that nests too deeply as a `ValueError`.
    ///
    /// # Arguments
    ///
    /// * `error` - The conversion error.
    ///
    /// # Returns
    ///
    /// The exception to throw.
    fn from(error: ConvertError) -> Self {
        match error {
            ConvertError::Unsupported(message) => PhpException::from(message),
            ConvertError::TooDeep => lindera_value_err(argument_too_deep_message()),
        }
    }
}

/// Converts a PHP Zval to a serde_json::Value.
///
/// # Arguments
///
/// * `zval` - PHP Zval to convert.
///
/// # Returns
///
/// A `serde_json::Value` representing the PHP value.
///
/// # Errors
///
/// Returns [`ConvertError::Unsupported`] if the PHP value type is not
/// supported, or [`ConvertError::TooDeep`] if it nests arrays more than
/// [`MAX_ARGUMENT_DEPTH`] levels deep.
pub fn zval_to_value(zval: &Zval) -> Result<Value, ConvertError> {
    value_at_depth(zval, 0)
}

/// Converts a PHP Zval that sits inside `depth` arrays.
///
/// # Arguments
///
/// * `zval` - PHP Zval to convert.
/// * `depth` - The number of arrays around `zval`.
///
/// # Returns
///
/// A `serde_json::Value` representing the PHP value, or the errors of
/// [`zval_to_value`].
fn value_at_depth(zval: &Zval, depth: usize) -> Result<Value, ConvertError> {
    if zval.is_null() {
        Ok(Value::Null)
    } else if zval.is_bool() {
        let b = bool::from_zval(zval).ok_or(ConvertError::Unsupported("failed to convert bool"))?;
        Ok(Value::Bool(b))
    } else if zval.is_long() {
        let i = i64::from_zval(zval).ok_or(ConvertError::Unsupported("failed to convert int"))?;
        Ok(Value::Number(serde_json::Number::from(i)))
    } else if zval.is_double() {
        let f = f64::from_zval(zval).ok_or(ConvertError::Unsupported("failed to convert float"))?;
        serde_json::Number::from_f64(f)
            .map(Value::Number)
            .ok_or(ConvertError::Unsupported("Invalid float value"))
    } else if zval.is_string() {
        let s =
            String::from_zval(zval).ok_or(ConvertError::Unsupported("failed to convert string"))?;
        Ok(Value::String(s))
    } else if zval.is_array() {
        let ht = zval
            .array()
            .ok_or(ConvertError::Unsupported("failed to get array"))?;
        hashtable_to_value(ht, depth)
    } else {
        Err(ConvertError::Unsupported("Unsupported PHP value type"))
    }
}

/// Converts a PHP ZendHashTable that sits inside `depth` arrays.
///
/// Detects whether the hashtable is a sequential array or an associative array
/// and converts accordingly.
///
/// # Arguments
///
/// * `ht` - PHP ZendHashTable to convert.
/// * `depth` - The number of arrays around `ht`.
///
/// # Returns
///
/// A `serde_json::Value` (Array or Object), or the errors of
/// [`zval_to_value`].
fn hashtable_to_value(ht: &ZendHashTable, depth: usize) -> Result<Value, ConvertError> {
    let depth = depth + 1;
    if depth > MAX_ARGUMENT_DEPTH {
        return Err(ConvertError::TooDeep);
    }

    // Check if it's a sequential array (all numeric keys starting from 0)
    let is_sequential = ht
        .iter()
        .enumerate()
        .all(|(i, (key, _))| matches!(key, ArrayKey::Long(idx) if usize::try_from(idx) == Ok(i)));

    if is_sequential {
        let mut arr = Vec::new();
        for (_, val) in ht.iter() {
            arr.push(value_at_depth(val, depth)?);
        }
        Ok(Value::Array(arr))
    } else {
        let mut map = Map::new();
        for (key, val) in ht.iter() {
            let key_str = match key {
                ArrayKey::String(s) => s,
                ArrayKey::Str(s) => s.to_string(),
                ArrayKey::Long(i) => i.to_string(),
                ArrayKey::ZendString(s) => s
                    .as_str()
                    .map_err(|_| ConvertError::Unsupported("failed to convert zend string key"))?
                    .to_string(),
            };
            map.insert(key_str, value_at_depth(val, depth)?);
        }
        Ok(Value::Object(map))
    }
}

/// An optional PHP argument: `null`, omitted, or a value that `T` accepts.
///
/// ext-php-rs before 0.16.3 converted an `Option<T>` argument with
/// `Some(T::from_zval(zval))`, so a value that `T` rejects became `None`
/// and the method silently ran with its default (lindera/lindera#1173).
/// This conversion fails for such a value instead, so ext-php-rs throws the
/// `TypeError` ("Invalid value given for argument") it throws for a required
/// parameter.
///
/// PHP sees the same parameter as for `Option<T>`, `?T $x = null`: since
/// ext-php-rs 0.16, nullability and the `null` default come from
/// [`FromZval::NULLABLE`] and [`FromZval::from_missing`], not from the
/// spelling `Option<...>`. ext-php-rs 0.16.3 rejects such values for
/// `Option<T>` itself
/// ([extphprs/ext-php-rs#819](https://github.com/extphprs/ext-php-rs/issues/819)),
/// so `Option<T>` can replace this type.
#[derive(Debug)]
pub struct Nullable<T>(Option<T>);

impl<T> Nullable<T> {
    /// Returns the argument's value.
    ///
    /// # Returns
    ///
    /// `None` for `null` or an omitted argument, the value otherwise.
    pub fn into_option(self) -> Option<T> {
        self.0
    }
}

impl<T> From<Option<T>> for Nullable<T> {
    /// Wraps a value for a Rust caller of a PHP method.
    ///
    /// # Arguments
    ///
    /// * `value` - `None` for `null`, or the value.
    ///
    /// # Returns
    ///
    /// The argument.
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

impl<'a, T: FromZval<'a>> FromZval<'a> for Nullable<T> {
    /// The PHP type of `T`; [`Self::NULLABLE`] adds the `?`.
    const TYPE: DataType = T::TYPE;

    /// Lets PHP callers pass `null` or omit the argument, as for `Option<T>`.
    const NULLABLE: bool = true;

    /// Reads `null` as `None` and any other value as `T`.
    ///
    /// # Arguments
    ///
    /// * `zval` - The PHP value.
    ///
    /// # Returns
    ///
    /// The argument, or `None` (the conversion fails) for a value that `T`
    /// does not accept.
    fn from_zval(zval: &'a Zval) -> Option<Self> {
        nullable_value(zval.is_null(), || T::from_zval(zval)).map(Self)
    }

    /// Reads an omitted argument as `None`.
    ///
    /// # Returns
    ///
    /// `Some` of an argument without a value.
    fn from_missing() -> Option<Self> {
        Some(Self(None))
    }
}

/// Decides the value of a [`Nullable`] argument.
///
/// # Arguments
///
/// * `is_null` - Whether the PHP value is `null`.
/// * `convert` - Converts the PHP value to `T`, giving `None` for a value
///   that `T` does not accept. Not called for `null`.
///
/// # Returns
///
/// `Some(None)` for `null`, `Some(Some(value))` for a value that `T`
/// accepts, and `None`, a failed conversion, for any other value.
fn nullable_value<T>(is_null: bool, convert: impl FnOnce() -> Option<T>) -> Option<Option<T>> {
    if is_null {
        Some(None)
    } else {
        convert().map(Some)
    }
}

/// A PHP `float` argument, which also accepts an `int`.
///
/// PHP widens an `int` passed to a `float` parameter, also under
/// `strict_types`, but ext-php-rs converts an `f64` argument from a PHP
/// `float` only.
#[derive(Debug, Clone, Copy)]
pub struct Float(f64);

impl Float {
    /// Returns the argument's value.
    ///
    /// # Returns
    ///
    /// The value as an `f64`.
    pub fn into_f64(self) -> f64 {
        self.0
    }
}

impl FromZval<'_> for Float {
    /// A PHP `float`.
    const TYPE: DataType = DataType::Double;

    /// Reads a PHP `float`, or an `int` widened to a float.
    ///
    /// # Arguments
    ///
    /// * `zval` - The PHP value.
    ///
    /// # Returns
    ///
    /// The argument, or `None` (the conversion fails) for a value of any
    /// other type.
    fn from_zval(zval: &Zval) -> Option<Self> {
        float_value(zval.double(), zval.long()).map(Self)
    }
}

/// Decides the value of a [`Float`] argument.
///
/// # Arguments
///
/// * `double` - The value if the PHP value is a `float`.
/// * `long` - The value if the PHP value is an `int`.
///
/// # Returns
///
/// The float, the int widened to a float, or `None` for a value of any other
/// type.
fn float_value(double: Option<f64>, long: Option<i64>) -> Option<f64> {
    double.or(long.map(|value| value as f64))
}

/// Converts an `int` argument to the type the core stores it as.
///
/// ext-php-rs reads a PHP `int` as an `i64`, and the core keeps most integer
/// settings in a narrower type, into which an `as` cast would wrap a value
/// that does not fit (lindera/lindera#1209).
///
/// # Arguments
///
/// * `name` - The parameter's name as PHP callers write it, without the `$`.
/// * `value` - The value passed.
///
/// # Returns
///
/// The value as `T`, or a `ValueError` that names the parameter and the
/// range of `T` when `T` cannot hold the value.
pub fn int_arg<T: IntegerArgument>(name: &str, value: i64) -> PhpResult<T> {
    integer_argument(name, value).map_err(lindera_value_err)
}

/// Converts an optional `int` argument with [`int_arg`].
///
/// # Arguments
///
/// * `name` - The parameter's name as PHP callers write it, without the `$`.
/// * `value` - The argument: `null`, omitted, or an `int`.
///
/// # Returns
///
/// `None` for `null` or an omitted argument, the value as `T` otherwise, or
/// the `ValueError` of [`int_arg`].
pub fn nullable_int_arg<T: IntegerArgument>(
    name: &str,
    value: Nullable<i64>,
) -> PhpResult<Option<T>> {
    optional_int(name, value.into_option()).map_err(lindera_value_err)
}

/// Converts an optional `int` argument to the type the core stores it as,
/// without building a PHP exception.
///
/// A constructor checks its arguments with this in a function that Rust
/// tests can call: building a PHP exception links symbols that only the PHP
/// runtime provides, which the test binary does not have.
///
/// # Arguments
///
/// * `name` - The parameter's name as PHP callers write it, without the `$`.
/// * `value` - The argument's value, `None` for `null` or an omitted
///   argument.
///
/// # Returns
///
/// `None` for `None`, the value as `T` otherwise, or the message of
/// [`integer_argument`] when `T` cannot hold the value.
pub fn optional_int<T: IntegerArgument>(
    name: &str,
    value: Option<i64>,
) -> Result<Option<T>, String> {
    value.map(|value| integer_argument(name, value)).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    // `Zval`s are not built here: setting one calls into the PHP runtime,
    // whose symbols this test binary does not link. PHPUnit
    // (ArgumentTypesTest) passes real PHP values.

    #[test]
    fn test_nullable_value_reads_null_as_none_without_converting() {
        let value = nullable_value::<i64>(true, || panic!("null must not be converted"));
        assert_eq!(value, Some(None));
    }

    #[test]
    fn test_nullable_value_keeps_a_value_that_t_accepts() {
        assert_eq!(nullable_value(false, || Some(5_i64)), Some(Some(5)));
    }

    #[test]
    fn test_nullable_value_fails_for_a_value_that_t_rejects() {
        assert_eq!(nullable_value::<i64>(false, || None), None);
    }

    #[test]
    fn test_nullable_is_nullable_and_optional_with_the_type_of_t() {
        const { assert!(<Nullable<i64> as FromZval>::NULLABLE) };
        assert_eq!(<Nullable<i64> as FromZval>::TYPE, DataType::Long);
        assert_eq!(<Nullable<bool> as FromZval>::TYPE, DataType::Bool);
        assert_eq!(<Nullable<String> as FromZval>::TYPE, DataType::String);
        assert_eq!(<Nullable<Float> as FromZval>::TYPE, DataType::Double);
        let missing = <Nullable<i64> as FromZval>::from_missing().map(Nullable::into_option);
        assert_eq!(missing, Some(None));
    }

    #[test]
    fn test_nullable_from_option() {
        assert_eq!(Nullable::from(Some(5_i64)).into_option(), Some(5));
        assert_eq!(Nullable::<i64>::from(None).into_option(), None);
    }

    #[test]
    fn test_float_value_takes_a_float() {
        assert_eq!(float_value(Some(0.5), None), Some(0.5));
    }

    #[test]
    fn test_float_value_widens_an_int() {
        assert_eq!(float_value(None, Some(1)), Some(1.0));
    }

    #[test]
    fn test_float_value_fails_for_other_types() {
        assert_eq!(float_value(None, None), None);
    }

    #[test]
    fn test_float_is_a_required_php_float() {
        assert_eq!(<Float as FromZval>::TYPE, DataType::Double);
        const { assert!(!<Float as FromZval>::NULLABLE) };
    }
}
