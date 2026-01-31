use wasm_bindgen::prelude::*;

/// Core segmenter for morphological analysis.
#[wasm_bindgen(js_name = "Segmenter")]
#[derive(Clone)]
pub struct JsSegmenter {
    pub(crate) _inner: lindera::segmenter::Segmenter,
}

#[wasm_bindgen]
impl JsSegmenter {
    // No public constructor for now as it's typically managed by Tokenizer
}
