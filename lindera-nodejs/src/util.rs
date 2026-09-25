//! Utility functions for data conversion.
//!
//! This module provides helper functions for working with JavaScript values
//! in the napi-rs context.

use lindera_binding_core::argument::{MAX_ARGUMENT_DEPTH, argument_too_deep_message};
use napi::bindgen_prelude::{Array, FromNapiValue, Object};
use napi::{Error, JsValue, Status, Unknown, ValueType, check_status, sys};
use serde_json::{Map, Value};

/// A JSON argument converted from a JavaScript value with a nesting limit.
///
/// napi-rs converts a `serde_json::Value` parameter by recursing into every
/// array and object without a limit, so a value that contains itself, or one
/// nested a few thousand levels deep, overflows the native stack and aborts
/// the process. This type walks the value the same way napi-rs does and
/// hands every non-container to napi-rs's own conversion, so valid arguments
/// convert exactly as before, but it counts the nesting and fails with an
/// ordinary JS error beyond [`MAX_ARGUMENT_DEPTH`].
pub struct JsonArg(pub Value);

impl FromNapiValue for JsonArg {
    /// Converts a JS value to JSON, failing when it nests too deeply.
    ///
    /// # Arguments
    ///
    /// * `env` - The environment of the current call.
    /// * `napi_val` - The JS value to convert.
    ///
    /// # Returns
    ///
    /// The JSON value, the error napi-rs raises for a value JSON cannot
    /// represent (such as a function or `undefined`), or an `InvalidArg`
    /// error when the value nests more than [`MAX_ARGUMENT_DEPTH`] levels.
    unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
        // SAFETY: napi-rs calls this with the environment and a value handle
        // of the current call, which is what `json_from_napi` requires.
        unsafe { json_from_napi(env, napi_val, 0) }.map(JsonArg)
    }
}

/// Converts a JS value to JSON, counting the containers around it.
///
/// Mirrors napi-rs's `serde_json::Value` conversion: arrays are read element
/// by element, objects through their enumerable string keys, and properties
/// whose value is `undefined` are skipped. Everything that is not an object
/// goes to napi-rs's own conversion, which gives the same values and error
/// messages as before.
///
/// # Arguments
///
/// * `env` - The environment of the current call.
/// * `napi_val` - The JS value to convert.
/// * `depth` - The number of containers that enclose `napi_val`.
///
/// # Returns
///
/// The JSON value, or an error for a value JSON cannot represent or one that
/// nests more than [`MAX_ARGUMENT_DEPTH`] levels.
///
/// # Safety
///
/// `env` and `napi_val` must be valid handles of the current call, as
/// napi-rs guarantees for [`FromNapiValue::from_napi_value`].
unsafe fn json_from_napi(
    env: sys::napi_env,
    napi_val: sys::napi_value,
    depth: usize,
) -> napi::Result<Value> {
    // SAFETY: the caller guarantees both handles are valid for this call.
    let value = unsafe { Unknown::from_napi_value(env, napi_val)? };
    if value.get_type()? != ValueType::Object {
        // SAFETY: as above. A non-object does not recurse.
        return unsafe { Value::from_napi_value(env, napi_val) };
    }

    let depth = depth + 1;
    if depth > MAX_ARGUMENT_DEPTH {
        return Err(Error::new(Status::InvalidArg, argument_too_deep_message()));
    }

    let mut is_array = false;
    // SAFETY: as above; `is_array` outlives the call.
    check_status!(
        unsafe { sys::napi_is_array(env, napi_val, &mut is_array) },
        "Failed to detect whether given js is an array"
    )?;

    if is_array {
        // SAFETY: as above, and the value was just checked to be an array.
        let array = unsafe { Array::from_napi_value(env, napi_val)? };
        let mut values = Vec::with_capacity(array.len() as usize);
        for index in 0..array.len() {
            let element = array.get::<Unknown>(index)?.ok_or_else(|| {
                Error::new(
                    Status::InvalidArg,
                    "Found inconsistent data type in Array<T> when converting to Rust Vec<T>",
                )
            })?;
            // SAFETY: the element handle belongs to the current call.
            values.push(unsafe { json_from_napi(env, element.raw(), depth)? });
        }
        Ok(Value::Array(values))
    } else {
        let object = Object::from_raw(env, napi_val);
        let mut map = Map::new();
        for key in Object::keys(&object)? {
            if let Some(property) = object.get::<Unknown>(&key)? {
                // SAFETY: the property handle belongs to the current call.
                map.insert(key, unsafe { json_from_napi(env, property.raw(), depth)? });
            }
        }
        Ok(Value::Object(map))
    }
}

/// Converts an optional serde_json::Value to a Value, defaulting to an empty object.
///
/// # Arguments
///
/// * `value` - Optional JSON value from JavaScript.
///
/// # Returns
///
/// The value if present, or an empty JSON object.
pub fn js_value_to_serde_value(value: Option<Value>) -> Value {
    value.unwrap_or_else(|| Value::Object(serde_json::Map::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_value_to_serde_value_none_returns_empty_object() {
        let result = js_value_to_serde_value(None);
        assert_eq!(result, Value::Object(serde_json::Map::new()));
    }

    #[test]
    fn test_js_value_to_serde_value_some_returns_value() {
        let input = Value::String("hello".to_string());
        let result = js_value_to_serde_value(Some(input.clone()));
        assert_eq!(result, input);
    }

    #[test]
    fn test_js_value_to_serde_value_some_object() {
        let mut map = serde_json::Map::new();
        map.insert("key".to_string(), Value::Number(42.into()));
        let input = Value::Object(map.clone());
        let result = js_value_to_serde_value(Some(input));
        assert_eq!(result, Value::Object(map));
    }

    #[test]
    fn test_js_value_to_serde_value_some_array() {
        let input = Value::Array(vec![Value::Bool(true), Value::Null]);
        let result = js_value_to_serde_value(Some(input.clone()));
        assert_eq!(result, input);
    }
}
