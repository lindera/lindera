use std::path::Path;

use wasm_bindgen::prelude::*;

use lindera::dictionary::{
    Dictionary, DictionaryBuilder, UserDictionary, load_dictionary as lindera_load_dictionary,
    load_user_dictionary as lindera_load_user_dictionary,
};

use crate::metadata::JsMetadata;

/// A morphological analysis dictionary.
#[wasm_bindgen(js_name = "Dictionary")]
#[derive(Clone)]
pub struct JsDictionary {
    pub(crate) inner: Dictionary,
}

#[wasm_bindgen]
impl JsDictionary {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.metadata.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn encoding(&self) -> String {
        self.inner.metadata.encoding.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> JsMetadata {
        JsMetadata {
            inner: self.inner.metadata.clone(),
        }
    }
}

/// A user-defined dictionary for custom words.
#[wasm_bindgen(js_name = "UserDictionary")]
#[derive(Clone)]
pub struct JsUserDictionary {
    pub(crate) inner: UserDictionary,
}

impl JsUserDictionary {
    pub fn new(inner: UserDictionary) -> Self {
        Self { inner }
    }
}

/// Loads a dictionary from the specified URI.
#[wasm_bindgen(js_name = "loadDictionary")]
pub fn load_dictionary(uri: &str) -> Result<JsDictionary, JsValue> {
    let dict = lindera_load_dictionary(uri).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsDictionary { inner: dict })
}

/// Loads a user dictionary from the specified URI.
#[wasm_bindgen(js_name = "loadUserDictionary")]
pub fn load_user_dictionary(uri: &str, metadata: JsMetadata) -> Result<JsUserDictionary, JsValue> {
    let dict = lindera_load_user_dictionary(uri, &metadata.inner)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsUserDictionary { inner: dict })
}

/// Builds a dictionary from source files.
#[wasm_bindgen(js_name = "buildDictionary")]
pub fn build_dictionary(
    input_dir: &str,
    output_dir: &str,
    metadata: JsMetadata,
) -> Result<(), JsValue> {
    let builder = DictionaryBuilder::new(metadata.inner);
    builder
        .build_dictionary(Path::new(input_dir), Path::new(output_dir))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(())
}

/// Builds a user dictionary from a CSV file.
#[wasm_bindgen(js_name = "buildUserDictionary")]
pub fn build_user_dictionary(
    input_file: &str,
    output_dir: &str,
    metadata: Option<JsMetadata>,
) -> Result<(), JsValue> {
    let meta = metadata.map(|m| m.inner).unwrap_or_default();
    let builder = DictionaryBuilder::new(meta);
    builder
        .build_user_dictionary(Path::new(input_file), Path::new(output_dir))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(())
}
