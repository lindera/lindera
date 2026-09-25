//! Utility functions for Ruby-Rust data conversion.
//!
//! This module provides helper functions for converting between Ruby objects
//! and Rust data structures, particularly for working with JSON-like data.

use lindera_binding_core::argument::{MAX_ARGUMENT_DEPTH, argument_too_deep_message};
use magnus::prelude::*;
use magnus::{Error, RArray, RHash, Ruby, TryConvert, Value};

/// Converts a Ruby value to a serde_json::Value.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `value` - Ruby value to convert.
///
/// # Returns
///
/// A `serde_json::Value` representing the Ruby value.
///
/// # Errors
///
/// Returns a `TypeError` if the Ruby value type is not supported, or if it
/// is a non-finite `Float` (NaN or Infinity), which JSON cannot represent,
/// and an `ArgumentError` if it nests arrays and hashes more than
/// [`MAX_ARGUMENT_DEPTH`] levels deep (as one that contains itself does).
pub fn rb_value_to_json(ruby: &Ruby, value: Value) -> Result<serde_json::Value, Error> {
    value_at_depth(ruby, value, 0)
}

/// Converts a Ruby value that sits inside `depth` arrays and hashes.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `value` - Ruby value to convert.
/// * `depth` - The number of arrays and hashes around `value`.
///
/// # Returns
///
/// A `serde_json::Value` representing the Ruby value, or the errors of
/// [`rb_value_to_json`].
fn value_at_depth(ruby: &Ruby, value: Value, depth: usize) -> Result<serde_json::Value, Error> {
    if value.is_nil() {
        Ok(serde_json::Value::Null)
    } else if value.is_kind_of(ruby.class_true_class())
        || value.is_kind_of(ruby.class_false_class())
    {
        let b: bool = TryConvert::try_convert(value).map_err(|e| {
            Error::new(
                ruby.exception_type_error(),
                format!("Failed to convert boolean: {e}"),
            )
        })?;
        Ok(serde_json::Value::Bool(b))
    } else if value.is_kind_of(ruby.class_integer()) {
        let i: i64 = TryConvert::try_convert(value).map_err(|e| {
            Error::new(
                ruby.exception_type_error(),
                format!("Failed to convert integer: {e}"),
            )
        })?;
        Ok(serde_json::Value::from(i))
    } else if value.is_kind_of(ruby.class_float()) {
        let f: f64 = TryConvert::try_convert(value).map_err(|e| {
            Error::new(
                ruby.exception_type_error(),
                format!("Failed to convert float: {e}"),
            )
        })?;
        // JSON has no NaN or Infinity; reject them rather than let them
        // silently turn into `null`, which callers read as "not set".
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| {
                Error::new(
                    ruby.exception_type_error(),
                    format!("Unsupported non-finite float: {f}"),
                )
            })
    } else if value.is_kind_of(ruby.class_string()) || value.is_kind_of(ruby.class_symbol()) {
        let s: String = TryConvert::try_convert(value).map_err(|e| {
            Error::new(
                ruby.exception_type_error(),
                format!("Failed to convert string: {e}"),
            )
        })?;
        Ok(serde_json::Value::String(s))
    } else if let Ok(arr) = RArray::try_convert(value) {
        array_at_depth(ruby, arr, depth)
    } else if let Ok(hash) = RHash::try_convert(value) {
        hash_at_depth(ruby, hash, depth)
    } else {
        Err(Error::new(
            ruby.exception_type_error(),
            format!("Unsupported Ruby object type: {}", unsafe {
                value.classname()
            }),
        ))
    }
}

/// Returns the depth of an array or hash that opens inside `depth` others.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `depth` - The number of arrays and hashes around the new one.
///
/// # Returns
///
/// The new depth, or an `ArgumentError` when it exceeds
/// [`MAX_ARGUMENT_DEPTH`].
fn container_depth(ruby: &Ruby, depth: usize) -> Result<usize, Error> {
    let depth = depth + 1;
    if depth > MAX_ARGUMENT_DEPTH {
        Err(Error::new(
            ruby.exception_arg_error(),
            argument_too_deep_message(),
        ))
    } else {
        Ok(depth)
    }
}

/// Converts a Ruby array that sits inside `depth` arrays and hashes.
///
/// The depth is counted here rather than in [`value_at_depth`] so that an
/// array obtained through `to_ary` counts too.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `array` - Ruby array to convert.
/// * `depth` - The number of arrays and hashes around `array`.
///
/// # Returns
///
/// A `serde_json::Value` representing the array, or the errors of
/// [`rb_value_to_json`].
fn array_at_depth(ruby: &Ruby, array: RArray, depth: usize) -> Result<serde_json::Value, Error> {
    let depth = container_depth(ruby, depth)?;
    let mut vec = Vec::new();
    for item in array.into_iter() {
        vec.push(value_at_depth(ruby, item, depth)?);
    }
    Ok(serde_json::Value::Array(vec))
}

/// Converts a Ruby hash to a serde_json::Value::Object.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `hash` - Ruby hash to convert.
///
/// # Returns
///
/// A `serde_json::Value` representing the hash, or the errors of
/// [`rb_value_to_json`]; the hash itself is the first level.
pub fn rb_hash_to_json(ruby: &Ruby, hash: RHash) -> Result<serde_json::Value, Error> {
    hash_at_depth(ruby, hash, 0)
}

/// Converts a Ruby hash that sits inside `depth` arrays and hashes.
///
/// The depth is counted here rather than in [`value_at_depth`] so that a
/// hash obtained through `to_hash` counts too.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `hash` - Ruby hash to convert.
/// * `depth` - The number of arrays and hashes around `hash`.
///
/// # Returns
///
/// A `serde_json::Value` representing the hash, or the errors of
/// [`rb_value_to_json`].
fn hash_at_depth(ruby: &Ruby, hash: RHash, depth: usize) -> Result<serde_json::Value, Error> {
    let depth = container_depth(ruby, depth)?;
    let mut map = serde_json::Map::new();
    hash.foreach(|key: String, value: Value| {
        let json_value = value_at_depth(ruby, value, depth)?;
        map.insert(key, json_value);
        Ok(magnus::r_hash::ForEach::Continue)
    })?;
    Ok(serde_json::Value::Object(map))
}

/// Converts a serde_json::Value to a Ruby value.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `value` - JSON value to convert.
///
/// # Returns
///
/// A Ruby `Value` representing the JSON value.
pub fn json_to_rb_value(ruby: &Ruby, value: &serde_json::Value) -> Result<Value, Error> {
    match value {
        serde_json::Value::Null => Ok(ruby.qnil().as_value()),
        serde_json::Value::Bool(b) => Ok(if *b {
            ruby.qtrue().as_value()
        } else {
            ruby.qfalse().as_value()
        }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(ruby.integer_from_i64(i).as_value())
            } else if let Some(f) = n.as_f64() {
                Ok(ruby.float_from_f64(f).as_value())
            } else {
                Err(Error::new(
                    ruby.exception_type_error(),
                    "Unsupported number type",
                ))
            }
        }
        serde_json::Value::String(s) => Ok(ruby.str_new(s).as_value()),
        serde_json::Value::Array(arr) => {
            let rb_arr = ruby.ary_new_capa(arr.len());
            for item in arr {
                rb_arr.push(json_to_rb_value(ruby, item)?)?;
            }
            Ok(rb_arr.as_value())
        }
        serde_json::Value::Object(obj) => {
            let rb_hash = ruby.hash_new();
            for (key, val) in obj {
                rb_hash.aset(ruby.str_new(key), json_to_rb_value(ruby, val)?)?;
            }
            Ok(rb_hash.as_value())
        }
    }
}
