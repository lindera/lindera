//! Token representation for morphological analysis results.
//!
//! Tokens cross into JavaScript as plain objects rather than
//! `#[wasm_bindgen]` class instances. A class instance keeps its data on the
//! Rust heap and relies on the JS side dropping it (explicitly, or through a
//! `FinalizationRegistry`) to release that memory, so a synchronous loop
//! that tokenizes without yielding accumulates memory. Building a plain
//! object instead means nothing is ever allocated on the Rust side for the
//! caller to release, and the result survives `JSON.stringify` /
//! `structuredClone` / worker transfer without conversion (#930).
//!
//! Field names are camelCase, matching the `lindera-nodejs` binding and what
//! the former `Token.toJSON()` already emitted.

use wasm_bindgen::prelude::*;

use lindera_binding_core::TokenView;

/// Converts a binding-core `TokenView` into a plain JS object.
///
/// # Arguments
///
/// * `view` - The token view produced by the binding-core tokenizer.
///
/// # Returns
///
/// A plain JS object with camelCase keys: `surface`, `byteStart`,
/// `byteEnd`, `position`, `wordId`, `isUnknown`, and `details`.
pub fn token_view_to_js(view: TokenView) -> JsValue {
    let js_obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&js_obj, &"surface".into(), &view.surface.into());
    let _ = js_sys::Reflect::set(
        &js_obj,
        &"byteStart".into(),
        &(view.byte_start as f64).into(),
    );
    let _ = js_sys::Reflect::set(&js_obj, &"byteEnd".into(), &(view.byte_end as f64).into());
    let _ = js_sys::Reflect::set(&js_obj, &"position".into(), &(view.position as f64).into());
    let _ = js_sys::Reflect::set(&js_obj, &"wordId".into(), &(view.word_id as f64).into());
    let _ = js_sys::Reflect::set(&js_obj, &"isUnknown".into(), &view.is_unknown.into());

    let js_details = js_sys::Array::new();
    for detail in view.details {
        js_details.push(&detail.into());
    }
    let _ = js_sys::Reflect::set(&js_obj, &"details".into(), &js_details.into());

    js_obj.into()
}
