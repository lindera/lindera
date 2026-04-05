//! Utility functions for Ruby-Rust data conversion.
//!
//! This module provides helper functions for converting between Ruby objects
//! and Rust data structures, particularly for working with JSON-like data.

use magnus::prelude::*;
use magnus::{Error, RArray, RHash, Ruby, TryConvert, Value};
use serde_json::json;

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
/// Returns an error if the Ruby value type is not supported.
pub fn rb_value_to_json(ruby: &Ruby, value: Value) -> Result<serde_json::Value, Error> {
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
        Ok(json!(f))
    } else if value.is_kind_of(ruby.class_string()) || value.is_kind_of(ruby.class_symbol()) {
        let s: String = TryConvert::try_convert(value).map_err(|e| {
            Error::new(
                ruby.exception_type_error(),
                format!("Failed to convert string: {e}"),
            )
        })?;
        Ok(serde_json::Value::String(s))
    } else if let Ok(arr) = RArray::try_convert(value) {
        rb_array_to_json(ruby, arr)
    } else if let Ok(hash) = RHash::try_convert(value) {
        rb_hash_to_json(ruby, hash)
    } else {
        Err(Error::new(
            ruby.exception_type_error(),
            format!("Unsupported Ruby object type: {}", unsafe {
                value.classname()
            }),
        ))
    }
}

/// Converts a Ruby array to a serde_json::Value::Array.
///
/// # Arguments
///
/// * `ruby` - Ruby runtime handle.
/// * `array` - Ruby array to convert.
///
/// # Returns
///
/// A `serde_json::Value` representing the array.
fn rb_array_to_json(ruby: &Ruby, array: RArray) -> Result<serde_json::Value, Error> {
    let mut vec = Vec::new();
    for item in array.into_iter() {
        vec.push(rb_value_to_json(ruby, item)?);
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
/// A `serde_json::Value` representing the hash.
pub fn rb_hash_to_json(ruby: &Ruby, hash: RHash) -> Result<serde_json::Value, Error> {
    let mut map = serde_json::Map::new();
    hash.foreach(|key: String, value: Value| {
        let json_value = rb_value_to_json(ruby, value)?;
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
