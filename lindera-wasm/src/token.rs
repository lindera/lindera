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

        let js_details = js_sys::Array::new();
        for detail in &self.details {
            js_details.push(&detail.clone().into());
        }
        let _ = js_sys::Reflect::set(&js_obj, &"details".into(), &js_details.into());

        js_obj.into()
    }
}

impl JsToken {
    pub fn from_token(mut token: Token) -> Self {
        let details = token.details().iter().map(|s| s.to_string()).collect();

        Self {
            surface: token.surface.to_string(),
            byte_start: token.byte_start,
            byte_end: token.byte_end,
            position: token.position,
            word_id: token.word_id.id,
            details,
        }
    }
}
