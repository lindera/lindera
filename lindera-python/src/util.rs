//! Utility functions for Python-Rust data conversion.
//!
//! This module provides helper functions for converting between Python objects
//! and Rust data structures, particularly for working with JSON-like data.

use std::collections::HashMap;

use lindera_binding_core::argument::{MAX_ARGUMENT_DEPTH, argument_too_deep_message};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyNone, PyString};
use serde_json::{Value, json};

/// Converts a Python object to a serde_json::Value.
///
/// # Arguments
///
/// * `value` - Python object to convert.
///
/// # Returns
///
/// A `serde_json::Value` representing the Python object.
///
/// # Errors
///
/// Returns a `TypeError` if the Python object type is not supported, or a
/// `ValueError` if it nests lists and dicts more than
/// [`MAX_ARGUMENT_DEPTH`] levels deep (as one that contains itself does).
pub fn pyany_to_value(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    value_at_depth(value, 0)
}

/// Converts a Python dictionary to a serde_json::Value.
///
/// # Arguments
///
/// * `pydict` - Python dictionary to convert.
///
/// # Returns
///
/// A `serde_json::Value` representing the dictionary.
///
/// # Errors
///
/// The same as [`pyany_to_value`]; the dictionary itself is the first level.
pub fn pydict_to_value(pydict: &Bound<'_, PyDict>) -> PyResult<Value> {
    dict_at_depth(pydict, 0)
}

/// Converts a Python object that sits inside `depth` lists and dicts.
///
/// # Arguments
///
/// * `value` - Python object to convert.
/// * `depth` - The number of lists and dicts around `value`.
///
/// # Returns
///
/// A `serde_json::Value` representing the Python object, or the errors of
/// [`pyany_to_value`].
fn value_at_depth(value: &Bound<'_, PyAny>, depth: usize) -> PyResult<Value> {
    if value.is_instance_of::<PyString>() {
        Ok(Value::from(value.extract::<String>()?))
    } else if value.is_instance_of::<PyBool>() {
        Ok(Value::from(value.extract::<bool>()?))
    } else if value.is_instance_of::<PyFloat>() {
        Ok(Value::from(value.extract::<f64>()?))
    } else if value.is_instance_of::<PyInt>() {
        Ok(Value::from(value.extract::<i64>()?))
    } else if value.is_instance_of::<PyList>() {
        list_at_depth(&value.extract::<Bound<'_, PyList>>()?, depth)
    } else if value.is_instance_of::<PyDict>() {
        dict_at_depth(&value.extract::<Bound<'_, PyDict>>()?, depth)
    } else if value.is_instance_of::<PyNone>() {
        Ok(Value::Null)
    } else {
        Err(PyErr::new::<PyTypeError, _>(format!(
            "Unsupported Python object: {value}"
        )))
    }
}

/// Returns the depth of a list or dict that opens inside `depth` others.
///
/// # Arguments
///
/// * `depth` - The number of lists and dicts around the new one.
///
/// # Returns
///
/// The new depth, or a `ValueError` when it exceeds [`MAX_ARGUMENT_DEPTH`].
fn container_depth(depth: usize) -> PyResult<usize> {
    let depth = depth + 1;
    if depth > MAX_ARGUMENT_DEPTH {
        Err(PyValueError::new_err(argument_too_deep_message()))
    } else {
        Ok(depth)
    }
}

/// Converts a Python list that sits inside `depth` lists and dicts.
///
/// # Arguments
///
/// * `pylist` - Python list to convert.
/// * `depth` - The number of lists and dicts around `pylist`.
///
/// # Returns
///
/// A JSON array, or the errors of [`pyany_to_value`].
fn list_at_depth(pylist: &Bound<'_, PyList>, depth: usize) -> PyResult<Value> {
    let depth = container_depth(depth)?;
    let mut vec: Vec<Value> = Vec::new();
    for value in pylist.into_iter() {
        vec.push(value_at_depth(&value, depth)?);
    }
    Ok(vec.into())
}

/// Converts a Python dictionary that sits inside `depth` lists and dicts.
///
/// # Arguments
///
/// * `pydict` - Python dictionary to convert.
/// * `depth` - The number of lists and dicts around `pydict`.
///
/// # Returns
///
/// A JSON object, or the errors of [`pyany_to_value`].
fn dict_at_depth(pydict: &Bound<'_, PyDict>, depth: usize) -> PyResult<Value> {
    let depth = container_depth(depth)?;
    let mut map: HashMap<String, Value> = HashMap::new();
    for (key, value) in pydict.into_iter() {
        map.insert(key.extract::<String>()?, value_at_depth(&value, depth)?);
    }
    Ok(json!(map))
}

/// Converts a serde_json::Value to a Python object.
///
/// # Arguments
///
/// * `py` - Python GIL token.
/// * `value` - JSON value to convert.
///
/// # Returns
///
/// A Python object representing the JSON value.
pub fn value_to_pydict(py: Python, value: &Value) -> PyResult<Py<PyAny>> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(PyBool::new(py, *b).into_pyobject(py)?.to_owned().into()),
        Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                Ok(i.into_pyobject(py)?.into())
            } else if let Some(f) = num.as_f64() {
                Ok(f.into_pyobject(py)?.into())
            } else {
                Err(PyTypeError::new_err("Unsupported number type"))
            }
        }
        Value::String(s) => Ok(PyString::new(py, s).into_pyobject(py)?.into()),
        Value::Array(arr) => {
            let py_list = PyList::empty(py);
            for item in arr {
                py_list.append(value_to_pydict(py, item)?)?;
            }
            Ok(py_list.into())
        }
        Value::Object(obj) => {
            let py_dict = PyDict::new(py);
            for (key, val) in obj {
                py_dict.set_item(key, value_to_pydict(py, val)?)?;
            }
            Ok(py_dict.into())
        }
    }
}

#[cfg(test)]
mod tests {}
