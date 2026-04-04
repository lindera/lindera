//! Utility functions for data conversion.
//!
//! This module provides helper functions for working with JavaScript values
//! in the napi-rs context.

use serde_json::Value;

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
