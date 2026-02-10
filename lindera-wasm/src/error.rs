use std::fmt;

use wasm_bindgen::prelude::*;

/// Error type for Lindera operations.
#[wasm_bindgen(js_name = "LinderaError")]
#[derive(Debug, Clone)]
pub struct JsLinderaError {
    #[wasm_bindgen(getter_with_clone)]
    pub message: String,
}

impl fmt::Display for JsLinderaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[wasm_bindgen]
impl JsLinderaError {
    #[wasm_bindgen(constructor)]
    pub fn new(message: String) -> Self {
        JsLinderaError { message }
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn js_to_string(&self) -> String {
        self.to_string()
    }
}
