//! Utility functions for Python-Rust data conversion.
//!
//! This module provides helper functions for converting between Python objects
//! and Rust data structures, particularly for working with JSON-like data.

use std::collections::HashMap;

use pyo3::exceptions::PyTypeError;
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
/// Returns an error if the Python object type is not supported.
pub fn pyany_to_value(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    if value.is_instance_of::<PyString>() {
        Ok(Value::from(value.extract::<String>()?))
    } else if value.is_instance_of::<PyBool>() {
        Ok(Value::from(value.extract::<bool>()?))
    } else if value.is_instance_of::<PyFloat>() {
        Ok(Value::from(value.extract::<f64>()?))
    } else if value.is_instance_of::<PyInt>() {
        Ok(Value::from(value.extract::<i64>()?))
    } else if value.is_instance_of::<PyList>() {
        pylist_to_value(&value.extract::<Bound<'_, PyList>>()?)
    } else if value.is_instance_of::<PyDict>() {
        pydict_to_value(&value.extract::<Bound<'_, PyDict>>()?)
    } else if value.is_instance_of::<PyNone>() {
        Ok(Value::Null)
    } else {
        Err(PyErr::new::<PyTypeError, _>(format!(
            "Unsupported Python object: {value}"
        )))
    }
}

fn pylist_to_value(pylist: &Bound<'_, PyList>) -> PyResult<Value> {
    let mut vec: Vec<Value> = Vec::new();
    for value in pylist.into_iter() {
        vec.push(pyany_to_value(&value)?);
    }
    Ok(vec.into())
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
pub fn pydict_to_value(pydict: &Bound<'_, PyDict>) -> PyResult<Value> {
    let mut map: HashMap<String, Value> = HashMap::new();
    for (key, value) in pydict.into_iter() {
        map.insert(key.extract::<String>()?, pyany_to_value(&value)?);
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
