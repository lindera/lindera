//! Utility functions for PHP-Rust data conversion.
//!
//! This module provides helper functions for converting between PHP Zval objects
//! and Rust data structures, particularly for working with JSON-like data.

use ext_php_rs::convert::FromZval;
use ext_php_rs::prelude::*;
use ext_php_rs::types::array::ArrayKey;
use ext_php_rs::types::{ZendHashTable, Zval};
use serde_json::{Map, Value};

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
/// Returns an error if the PHP value type is not supported.
pub fn zval_to_value(zval: &Zval) -> PhpResult<Value> {
    if zval.is_null() {
        Ok(Value::Null)
    } else if zval.is_bool() {
        let b = bool::from_zval(zval).ok_or("failed to convert bool")?;
        Ok(Value::Bool(b))
    } else if zval.is_long() {
        let i = i64::from_zval(zval).ok_or("failed to convert int")?;
        Ok(Value::Number(serde_json::Number::from(i)))
    } else if zval.is_double() {
        let f = f64::from_zval(zval).ok_or("failed to convert float")?;
        serde_json::Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| "Invalid float value".into())
    } else if zval.is_string() {
        let s = String::from_zval(zval).ok_or("failed to convert string")?;
        Ok(Value::String(s))
    } else if zval.is_array() {
        let ht = zval.array().ok_or("failed to get array")?;
        hashtable_to_value(ht)
    } else {
        Err("Unsupported PHP value type".into())
    }
}

/// Converts a PHP ZendHashTable to a serde_json::Value.
///
/// Detects whether the hashtable is a sequential array or an associative array
/// and converts accordingly.
///
/// # Arguments
///
/// * `ht` - PHP ZendHashTable to convert.
///
/// # Returns
///
/// A `serde_json::Value` (Array or Object).
pub fn hashtable_to_value(ht: &ZendHashTable) -> PhpResult<Value> {
    // Check if it's a sequential array (all numeric keys starting from 0)
    let is_sequential = ht
        .iter()
        .enumerate()
        .all(|(i, (key, _))| matches!(key, ArrayKey::Long(idx) if idx as usize == i));

    if is_sequential {
        let mut arr = Vec::new();
        for (_, val) in ht.iter() {
            arr.push(zval_to_value(val)?);
        }
        Ok(Value::Array(arr))
    } else {
        let mut map = Map::new();
        for (key, val) in ht.iter() {
            let key_str = match key {
                ArrayKey::String(s) => s,
                ArrayKey::Str(s) => s.to_string(),
                ArrayKey::Long(i) => i.to_string(),
            };
            map.insert(key_str, zval_to_value(val)?);
        }
        Ok(Value::Object(map))
    }
}
