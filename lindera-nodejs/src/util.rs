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
