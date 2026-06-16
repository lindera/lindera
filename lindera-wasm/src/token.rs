use wasm_bindgen::prelude::*;

use lindera::token::Token;

/// Token object wrapping the Rust Token data.
///
/// This class provides robust access to token field and details.
#[wasm_bindgen(js_name = "Token")]
pub struct JsToken {
    /// Surface form of the token.
    #[wasm_bindgen(getter_with_clone)]
    pub surface: String,

    /// Start byte position in the original text.
    pub byte_start: usize,

    /// End byte position in the original text.
    pub byte_end: usize,

    /// Position index of the token.
    pub position: usize,

    /// Word ID in the dictionary.
    pub word_id: u32,

    /// Whether this token is an unknown word (not found in the dictionary).
    pub is_unknown: bool,

    /// Morphological details of the token.
    #[wasm_bindgen(getter_with_clone)]
    pub details: Vec<String>,
}

#[wasm_bindgen(js_class = "Token")]
impl JsToken {
    /// Returns the detail at the specified index.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the detail to retrieve.
    ///
    /// # Returns
    ///
    /// The detail string if found, otherwise undefined.
    #[wasm_bindgen(js_name = "getDetail")]
    pub fn get_detail(&self, index: usize) -> Option<String> {
        self.details.get(index).cloned()
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> JsValue {
        let js_obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&js_obj, &"surface".into(), &self.surface.clone().into());
        let _ = js_sys::Reflect::set(
            &js_obj,
            &"byteStart".into(),
            &(self.byte_start as f64).into(),
        );
        let _ = js_sys::Reflect::set(&js_obj, &"byteEnd".into(), &(self.byte_end as f64).into());
        let _ = js_sys::Reflect::set(&js_obj, &"position".into(), &(self.position as f64).into());
        let _ = js_sys::Reflect::set(&js_obj, &"wordId".into(), &(self.word_id as f64).into());
        let _ = js_sys::Reflect::set(&js_obj, &"isUnknown".into(), &self.is_unknown.into());

        let js_details = js_sys::Array::new();
        for detail in &self.details {
            js_details.push(&detail.clone().into());
        }
        let _ = js_sys::Reflect::set(&js_obj, &"details".into(), &js_details.into());

        js_obj.into()
    }
}

impl JsToken {
    pub fn from_token(token: Token) -> Self {
        Self::from_view(lindera_binding_core::TokenView::from_token(token))
    }

    /// Creates a `JsToken` from a binding-core `TokenView`.
    pub fn from_view(view: lindera_binding_core::TokenView) -> Self {
        Self {
            surface: view.surface,
            byte_start: view.byte_start,
            byte_end: view.byte_end,
            position: view.position,
            word_id: view.word_id,
            is_unknown: view.is_unknown,
            details: view.details,
        }
    }
}
