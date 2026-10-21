//! Utility functions for PHP-Rust data conversion.
//!
//! This module provides helper functions for converting between PHP Zval objects
//! and Rust data structures, particularly for working with JSON-like data, and
//! [`int_arg`], which checks an `int` argument's range.

use ext_php_rs::convert::FromZval;
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
    value: Option<i64>,
) -> PhpResult<Option<T>> {
    optional_int(name, value).map_err(lindera_value_err)
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
