//! # lindera-wasm
//!
//! WebAssembly bindings for [Lindera](https://github.com/lindera/lindera), a morphological analysis library.
//!
//! This crate provides WASM bindings that enable Japanese, Korean, and Chinese text tokenization
//! in web browsers and Node.js environments.
//!
//! ## Features
//!
//! - **Multiple dictionaries**: IPADIC, UniDic (Japanese), ko-dic (Korean), CC-CEDICT (Chinese)
//! - **Flexible tokenization modes**: Normal and decompose modes
//! - **Character filters**: Unicode normalization and more
//! - **Token filters**: Lowercase, compound word handling, number normalization
//! - **Custom user dictionaries**: Support for user-defined dictionaries
//!
//! ## Usage
//!
//! ### Web (Browser)
//!
//! ```javascript
//! import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-web-ipadic';
//!
//! __wbg_init().then(() => {
//!     const builder = new TokenizerBuilder();
//!     builder.setDictionary("embedded://ipadic");
//!     builder.setMode("normal");
//!
//!     const tokenizer = builder.build();
//!     const tokens = tokenizer.tokenize("関西国際空港");
//!     console.log(tokens);
//! });
//! ```
//!
//! ### Node.js
//!
//! ```javascript
//! const { TokenizerBuilder } = require('lindera-wasm-nodejs-ipadic');
//!
//! const builder = new TokenizerBuilder();
//! builder.setDictionary("embedded://ipadic");
//! builder.setMode("normal");
//!
//! const tokenizer = builder.build();
//! const tokens = tokenizer.tokenize("関西国際空港");
//! console.log(tokens);
//! ```

use std::str::FromStr;

use serde_json::Value;
use wasm_bindgen::prelude::*;

use lindera::mode::Mode;
use lindera::token::Token;
use lindera::tokenizer::{
    Tokenizer as LinderaTokenizer, TokenizerBuilder as LinderaTokenizerBuilder,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Gets the version of the lindera-wasm library.
///
/// # Returns
///
/// The version string of the library (e.g., "1.0.0").
///
/// # Examples
///
/// ```javascript
/// import { getVersion } from 'lindera-wasm';
/// console.log(getVersion()); // "1.0.0"
/// ```
#[wasm_bindgen(js_name = "getVersion")]
pub fn get_version() -> String {
    VERSION.to_string()
}

/// Converts snake_case strings to camelCase.
///
/// This is used internally to convert Rust field names to JavaScript-friendly camelCase.
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Converts a serde_json::Value to a JsValue recursively.
///
/// This function handles conversion of various JSON types to their JavaScript equivalents,
/// including objects, arrays, strings, numbers, booleans, and null values.
fn value_to_js(value: Value) -> Result<JsValue, JsValue> {
    match value {
        Value::String(s) => Ok(JsValue::from_str(s.as_str())),
        Value::Number(n) => {
            if let Some(i) = n.as_u64() {
                Ok(JsValue::from_f64(i as f64))
            } else if let Some(i) = n.as_i64() {
                Ok(JsValue::from_f64(i as f64))
            } else if let Some(f) = n.as_f64() {
                Ok(JsValue::from_f64(f))
            } else {
                Ok(JsValue::from_str(&n.to_string()))
            }
        }
        Value::Bool(b) => Ok(JsValue::from_bool(b)),
        Value::Null => Ok(JsValue::NULL),
        Value::Array(arr) => {
            let js_arr = js_sys::Array::new();
            for item in arr {
                js_arr.push(&value_to_js(item)?);
            }
            Ok(js_arr.into())
        }
        Value::Object(map) => {
            let js_obj = js_sys::Object::new();
            for (key, val) in map {
                // Change key to camel case
                let js_key = JsValue::from_str(to_camel_case(&key).as_str());
                let js_val = value_to_js(val)?;
                js_sys::Reflect::set(&js_obj, &js_key, &js_val)
                    .map_err(|e| JsValue::from_str(&format!("Failed to set property: {e:?}")))?;
            }
            Ok(js_obj.into())
        }
    }
}

/// Converts a vector of tokens to a JavaScript array of objects.
///
/// Each token is converted to a JavaScript object with camelCase field names.
fn convert_to_js_objects(tokens: Vec<Token>) -> Result<JsValue, JsValue> {
    let js_array = js_sys::Array::new();
    for mut token in tokens {
        js_array.push(&value_to_js(token.as_value())?);
    }

    Ok(js_array.into())
}

/// Builder for creating a [`Tokenizer`] instance.
///
/// `TokenizerBuilder` provides a fluent API for configuring and building a tokenizer
/// with various options such as dictionary selection, tokenization mode, character filters,
/// and token filters.
///
/// # Examples
///
/// ```javascript
/// const builder = new TokenizerBuilder();
/// builder.setDictionary("embedded://ipadic");
/// builder.setMode("normal");
/// builder.setKeepWhitespace(false);
/// builder.appendCharacterFilter("unicode_normalize", { "kind": "nfkc" });
/// builder.appendTokenFilter("lowercase");
///
/// const tokenizer = builder.build();
/// ```
#[wasm_bindgen]
pub struct TokenizerBuilder {
    inner: LinderaTokenizerBuilder,
}

#[wasm_bindgen]
impl TokenizerBuilder {
    /// Creates a new `TokenizerBuilder` instance.
    ///
    /// # Returns
    ///
    /// A new `TokenizerBuilder` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder cannot be initialized.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const builder = new TokenizerBuilder();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let inner =
            LinderaTokenizerBuilder::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self { inner })
    }

    /// Builds and returns a configured [`Tokenizer`] instance.
    ///
    /// This method consumes the builder and creates the final tokenizer with all
    /// configured settings.
    ///
    /// # Returns
    ///
    /// A configured `Tokenizer` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokenizer cannot be built with the current configuration.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const builder = new TokenizerBuilder();
    /// builder.setDictionary("embedded://ipadic");
    /// const tokenizer = builder.build();
    /// ```
    pub fn build(self) -> Result<Tokenizer, JsValue> {
        let inner = self
            .inner
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Tokenizer { inner })
    }

    /// Sets the tokenization mode.
    ///
    /// # Parameters
    ///
    /// - `mode`: The tokenization mode. Valid values are:
    ///   - `"normal"`: Standard tokenization
    ///   - `"decompose"`: Decomposes compound words into their components
    ///
    /// # Errors
    ///
    /// Returns an error if the mode string is invalid.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// builder.setMode("normal");
    /// // or
    /// builder.setMode("decompose");
    /// ```
    #[wasm_bindgen(js_name = "setMode")]
    pub fn set_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        let m = Mode::from_str(mode).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.inner.set_segmenter_mode(&m);

        Ok(())
    }

    /// Sets the dictionary to use for tokenization.
    ///
    /// # Parameters
    ///
    /// - `uri`: The dictionary URI. Valid embedded dictionaries are:
    ///   - `"embedded://ipadic"`: Japanese IPADIC dictionary
    ///   - `"embedded://unidic"`: Japanese UniDic dictionary
    ///   - `"embedded://ko-dic"`: Korean ko-dic dictionary
    ///   - `"embedded://cc-cedict"`: Chinese CC-CEDICT dictionary
    ///
    /// # Examples
    ///
    /// ```javascript
    /// builder.setDictionary("embedded://ipadic");
    /// ```
    #[wasm_bindgen(js_name = "setDictionary")]
    pub fn set_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.inner.set_segmenter_dictionary(uri);

        Ok(())
    }

    /// Sets a user-defined dictionary.
    ///
    /// User dictionaries allow you to add custom words and their properties
    /// to supplement the main dictionary.
    ///
    /// # Parameters
    ///
    /// - `uri`: The URI to the user dictionary file.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// builder.setUserDictionary("path/to/user_dict.csv");
    /// ```
    #[wasm_bindgen(js_name = "setUserDictionary")]
    pub fn set_user_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.inner.set_segmenter_user_dictionary(uri);

        Ok(())
    }

    /// Sets whether to keep whitespace tokens in the output.
    ///
    /// # Parameters
    ///
    /// - `keep`: If `true`, whitespace tokens are preserved; if `false`, they are removed.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// builder.setKeepWhitespace(false); // Remove whitespace tokens
    /// // or
    /// builder.setKeepWhitespace(true);  // Keep whitespace tokens
    /// ```
    #[wasm_bindgen(js_name = "setKeepWhitespace")]
    pub fn set_keep_whitespace(&mut self, keep: bool) -> Result<(), JsValue> {
        self.inner.set_segmenter_keep_whitespace(keep);

        Ok(())
    }

    /// Appends a character filter to the tokenization pipeline.
    ///
    /// Character filters transform the input text before tokenization.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the character filter (e.g., `"unicode_normalize"`).
    /// - `args`: A JavaScript object containing filter-specific arguments.
    ///
    /// # Errors
    ///
    /// Returns an error if the arguments cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// builder.appendCharacterFilter("unicode_normalize", { "kind": "nfkc" });
    /// ```
    #[wasm_bindgen(js_name = "appendCharacterFilter")]
    pub fn append_character_filter(&mut self, name: &str, args: JsValue) -> Result<(), JsValue> {
        let a = serde_wasm_bindgen::from_value::<Value>(args)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        self.inner.append_character_filter(name, &a);

        Ok(())
    }

    /// Appends a token filter to the tokenization pipeline.
    ///
    /// Token filters transform or filter the tokens after tokenization.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the token filter (e.g., `"lowercase"`, `"japanese_number"`).
    /// - `args`: A JavaScript object containing filter-specific arguments.
    ///
    /// # Errors
    ///
    /// Returns an error if the arguments cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// builder.appendTokenFilter("lowercase");
    /// builder.appendTokenFilter("japanese_number", { "tags": ["名詞,数"] });
    /// ```
    #[wasm_bindgen(js_name = "appendTokenFilter")]
    pub fn append_token_filter(&mut self, name: &str, args: JsValue) -> Result<(), JsValue> {
        let a = serde_wasm_bindgen::from_value::<Value>(args)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        self.inner.append_token_filter(name, &a);

        Ok(())
    }
}

/// A tokenizer for morphological analysis.
///
/// The `Tokenizer` performs text tokenization based on the configuration
/// provided by [`TokenizerBuilder`].
///
/// # Examples
///
/// ```javascript
/// const builder = new TokenizerBuilder();
/// builder.setDictionary("embedded://ipadic");
/// builder.setMode("normal");
///
/// const tokenizer = builder.build();
/// const tokens = tokenizer.tokenize("関西国際空港");
/// console.log(tokens);
/// // Output: [
/// //   { surface: "関西国際空港", ... },
/// //   ...
/// // ]
/// ```
#[wasm_bindgen]
pub struct Tokenizer {
    inner: LinderaTokenizer,
}

#[wasm_bindgen]
impl Tokenizer {
    /// Tokenizes the input text.
    ///
    /// Analyzes the input text and returns an array of token objects. Each token
    /// contains information such as surface form, part-of-speech tags, reading, etc.
    /// Field names in the returned objects are in camelCase.
    ///
    /// # Parameters
    ///
    /// - `input_text`: The text to tokenize.
    ///
    /// # Returns
    ///
    /// A JavaScript array of token objects. Each token object contains:
    /// - `surface`: The surface form of the token
    /// - `pos`: Part-of-speech tags
    /// - Additional language-specific fields
    ///
    /// # Errors
    ///
    /// Returns an error if tokenization fails.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const tokens = tokenizer.tokenize("東京都に行く");
    /// tokens.forEach(token => {
    ///     console.log(token.surface, token.pos);
    /// });
    /// ```
    pub fn tokenize(&self, input_text: &str) -> Result<JsValue, JsValue> {
        let tokens = self
            .inner
            .tokenize(input_text)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let js_value = convert_to_js_objects(tokens)?;

        Ok(js_value)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize() {
        use crate::TokenizerBuilder;
        use serde_json::Value;

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let t = tokenizer.tokenize("関西国際空港限定トートバッグ").unwrap();
        let tokens: Vec<Value> = serde_wasm_bindgen::from_value(t).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].get("surface").unwrap(), "関西国際空港");
        assert_eq!(tokens[1].get("surface").unwrap(), "限定");
        assert_eq!(tokens[2].get("surface").unwrap(), "トートバッグ");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_camel_case() {
        use crate::to_camel_case;

        assert_eq!(to_camel_case("a"), "a");
        assert_eq!(to_camel_case("a_b"), "aB");
        assert_eq!(to_camel_case("a_b_c"), "aBC");
        assert_eq!(to_camel_case("a_b_c_d"), "aBCD");
        assert_eq!(to_camel_case("a_b_c_d_e"), "aBCDE");
    }
}
