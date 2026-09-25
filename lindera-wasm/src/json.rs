//! Depth-limited conversion of JS values to JSON.
//!
//! `serde_wasm_bindgen::from_value::<serde_json::Value>` recurses into every
//! array, object and `Map` without a limit. A value that contains itself, or
//! one nested a few thousand levels deep, exhausts the wasm stack; the trap
//! leaves the module's stack pointer and borrow flags behind, so every later
//! call into the module fails too. [`json_from_js`] walks the value with the
//! same deserializer, so valid values convert exactly as before, but counts
//! the containers and fails with an ordinary error beyond
//! [`MAX_ARGUMENT_DEPTH`].

use std::fmt;

use lindera_binding_core::argument::{MAX_ARGUMENT_DEPTH, argument_too_deep_message};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use wasm_bindgen::JsValue;

/// Converts a JS value to JSON, failing when it nests too deeply.
///
/// # Arguments
///
/// * `value` - The JS value to convert.
///
/// # Returns
///
/// The JSON value, or an error when the value has no JSON form or nests more
/// than [`MAX_ARGUMENT_DEPTH`] levels.
pub(crate) fn json_from_js(value: JsValue) -> Result<Value, serde_wasm_bindgen::Error> {
    JsonSeed { depth: 0 }.deserialize(serde_wasm_bindgen::Deserializer::from(value))
}

/// Deserializes one JSON value that sits inside `depth` containers.
#[derive(Clone, Copy)]
struct JsonSeed {
    /// The number of containers around the value.
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for JsonSeed {
    type Value = Value;

    /// Reads any value, as serde_json's own `Value` does.
    ///
    /// # Arguments
    ///
    /// * `deserializer` - The deserializer holding the value.
    ///
    /// # Returns
    ///
    /// The JSON value, or the deserializer's error.
    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(JsonVisitor { depth: self.depth })
    }
}

/// Builds a JSON value the way serde_json's `ValueVisitor` does, counting
/// the containers.
struct JsonVisitor {
    /// The number of containers around the value.
    depth: usize,
}

impl JsonVisitor {
    /// Returns the depth of a container opened at this value.
    ///
    /// # Returns
    ///
    /// The depth, or an error when it exceeds [`MAX_ARGUMENT_DEPTH`].
    fn container_depth<E: de::Error>(&self) -> Result<usize, E> {
        let depth = self.depth + 1;
        if depth > MAX_ARGUMENT_DEPTH {
            Err(E::custom(argument_too_deep_message()))
        } else {
            Ok(depth)
        }
    }
}

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Value;

    /// Describes the expected input, in serde_json's words.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    /// Converts a boolean.
    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    /// Converts a signed integer.
    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    /// Converts an unsigned integer.
    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    /// Converts a float; `NaN` and the infinities become `null`.
    fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
        Ok(Number::from_f64(value).map_or(Value::Null, Value::Number))
    }

    /// Converts a borrowed string.
    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    /// Converts an owned string.
    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    /// Converts an absent value to `null`.
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    /// Converts a present optional value.
    fn visit_some<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        JsonSeed { depth: self.depth }.deserialize(deserializer)
    }

    /// Converts `null` and `undefined` to `null`.
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    /// Converts an array, one level deeper.
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let depth = self.container_depth()?;
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(JsonSeed { depth })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    /// Converts an object or a `Map`, one level deeper.
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let depth = self.container_depth()?;
        let mut values = Map::new();
        while let Some(key) = map.next_key_seed(KeySeed)? {
            let value = map.next_value_seed(JsonSeed { depth })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

/// Deserializes an object or `Map` key, accepting only strings.
///
/// serde-wasm-bindgen reports a non-string key with the JS `debugString`
/// helper, which recurses without a limit, so a `Map` keyed by an array that
/// contains itself would crash while building the error message. Reading the
/// key with `deserialize_any` rejects any non-string without looking inside.
struct KeySeed;

impl<'de> DeserializeSeed<'de> for KeySeed {
    type Value = String;

    /// Reads a key.
    ///
    /// # Arguments
    ///
    /// * `deserializer` - The deserializer holding the key.
    ///
    /// # Returns
    ///
    /// The key, or an error when it is not a string.
    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<String, D::Error> {
        deserializer.deserialize_any(KeyVisitor)
    }
}

/// Accepts a string key and rejects everything else.
struct KeyVisitor;

impl Visitor<'_> for KeyVisitor {
    type Value = String;

    /// Describes the expected input.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string key")
    }

    /// Accepts a borrowed string key.
    fn visit_str<E>(self, value: &str) -> Result<String, E> {
        Ok(value.to_owned())
    }

    /// Accepts an owned string key.
    fn visit_string<E>(self, value: String) -> Result<String, E> {
        Ok(value)
    }
}
