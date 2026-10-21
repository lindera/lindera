//! Helpers for the tests that call the bindings through JS.
//!
//! A test that calls an exported function from Rust does not run the JS glue
//! wasm-bindgen generates, and the glue is where a class argument taken by
//! value is destroyed (#1238). These helpers call the way a JS caller does,
//! so the glue runs.

use wasm_bindgen::{JsCast, JsValue};

/// Calls the method `name` of `target` through JS.
///
/// # Arguments
///
/// * `target` - The object, such as a `TokenizerBuilder` converted with
///   `JsValue::from`.
/// * `name` - The method's JS name, such as `"setDictionaryInstance"`.
/// * `args` - The arguments.
///
/// # Returns
///
/// What the method returns, or what it or its glue throws.
pub(crate) fn call_method(
    target: &JsValue,
    name: &str,
    args: &[&JsValue],
) -> Result<JsValue, JsValue> {
    let method: js_sys::Function = js_sys::Reflect::get(target, &name.into())?.dyn_into()?;
    js_sys::Reflect::apply(&method, target, &args.iter().copied().collect())
}

/// Calls the constructor of the class of `instance` through JS, as `new`
/// does.
///
/// # Arguments
///
/// * `instance` - An object of the class, such as a `Tokenizer` converted
///   with `JsValue::from`.
/// * `args` - The arguments of the constructor.
///
/// # Returns
///
/// The new object, or what the constructor or its glue throws.
pub(crate) fn construct_like(instance: &JsValue, args: &[&JsValue]) -> Result<JsValue, JsValue> {
    let class: js_sys::Function =
        js_sys::Reflect::get(instance, &"constructor".into())?.dyn_into()?;
    js_sys::Reflect::construct(&class, &args.iter().copied().collect())
}

/// Reads the strings of a JS array, such as the result of
/// `tokenizeSurfaces`.
///
/// # Arguments
///
/// * `array` - The array.
///
/// # Returns
///
/// The strings, with the debug form of an element that is not a string.
pub(crate) fn strings(array: &JsValue) -> Vec<String> {
    js_sys::Array::from(array)
        .iter()
        .map(|value| value.as_string().unwrap_or_else(|| format!("{value:?}")))
        .collect()
}
