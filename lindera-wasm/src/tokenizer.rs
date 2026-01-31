use std::str::FromStr;

use serde_json::Value;
use wasm_bindgen::prelude::*;

use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::{
    Tokenizer as LinderaTokenizer, TokenizerBuilder as LinderaTokenizerBuilder,
};

use crate::dictionary::{JsDictionary, JsUserDictionary};
use crate::token::JsToken;

/// Builder for creating a [`Tokenizer`] instance.
///
/// `TokenizerBuilder` provides a fluent API for configuring and building a tokenizer
/// with various options such as dictionary selection, tokenization mode, character filters,
/// and token filters.
#[wasm_bindgen]
pub struct TokenizerBuilder {
    inner: LinderaTokenizerBuilder,
}

#[wasm_bindgen]
impl TokenizerBuilder {
    /// Creates a new `TokenizerBuilder` instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let inner =
            LinderaTokenizerBuilder::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self { inner })
    }

    /// Builds and returns a configured [`Tokenizer`] instance.
    pub fn build(self) -> Result<Tokenizer, JsValue> {
        let inner = self
            .inner
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Tokenizer { inner })
    }

    /// Sets the tokenization mode.
    #[wasm_bindgen(js_name = "setMode")]
    pub fn set_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        let m = Mode::from_str(mode).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.inner.set_segmenter_mode(&m);

        Ok(())
    }

    /// Sets the dictionary to use for tokenization.
    #[wasm_bindgen(js_name = "setDictionary")]
    pub fn set_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.inner.set_segmenter_dictionary(uri);

        Ok(())
    }

    /// Sets a user-defined dictionary.
    #[wasm_bindgen(js_name = "setUserDictionary")]
    pub fn set_user_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.inner.set_segmenter_user_dictionary(uri);

        Ok(())
    }

    /// Sets whether to keep whitespace tokens in the output.
    #[wasm_bindgen(js_name = "setKeepWhitespace")]
    pub fn set_keep_whitespace(&mut self, keep: bool) -> Result<(), JsValue> {
        self.inner.set_segmenter_keep_whitespace(keep);

        Ok(())
    }

    /// Appends a character filter to the tokenization pipeline.
    #[wasm_bindgen(js_name = "appendCharacterFilter")]
    pub fn append_character_filter(&mut self, name: &str, args: JsValue) -> Result<(), JsValue> {
        let a = if args.is_undefined() || args.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_wasm_bindgen::from_value::<Value>(args)
                .map_err(|e| JsValue::from_str(&e.to_string()))?
        };

        self.inner.append_character_filter(name, &a);

        Ok(())
    }

    /// Appends a token filter to the tokenization pipeline.
    #[wasm_bindgen(js_name = "appendTokenFilter")]
    pub fn append_token_filter(&mut self, name: &str, args: JsValue) -> Result<(), JsValue> {
        let a = if args.is_undefined() || args.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_wasm_bindgen::from_value::<Value>(args)
                .map_err(|e| JsValue::from_str(&e.to_string()))?
        };

        self.inner.append_token_filter(name, &a);

        Ok(())
    }

    // Python-style aliases (snake_case)
    #[wasm_bindgen(js_name = "set_mode")]
    pub fn py_set_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        self.set_mode(mode)
    }

    #[wasm_bindgen(js_name = "set_dictionary")]
    pub fn py_set_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.set_dictionary(uri)
    }

    #[wasm_bindgen(js_name = "set_user_dictionary")]
    pub fn py_set_user_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.set_user_dictionary(uri)
    }

    #[wasm_bindgen(js_name = "set_keep_whitespace")]
    pub fn py_set_keep_whitespace(&mut self, keep: bool) -> Result<(), JsValue> {
        self.set_keep_whitespace(keep)
    }

    #[wasm_bindgen(js_name = "append_character_filter")]
    pub fn py_append_character_filter(&mut self, name: &str, args: JsValue) -> Result<(), JsValue> {
        self.append_character_filter(name, args)
    }

    #[wasm_bindgen(js_name = "append_token_filter")]
    pub fn py_append_token_filter(&mut self, name: &str, args: JsValue) -> Result<(), JsValue> {
        self.append_token_filter(name, args)
    }
}

/// A tokenizer for morphological analysis.
#[wasm_bindgen]
pub struct Tokenizer {
    inner: LinderaTokenizer,
}

#[wasm_bindgen]
impl Tokenizer {
    #[wasm_bindgen(constructor)]
    pub fn new(
        dictionary: JsDictionary,
        mode: Option<String>,
        user_dictionary: Option<JsUserDictionary>,
    ) -> Result<Tokenizer, JsValue> {
        let m = if let Some(mode_str) = mode {
            Mode::from_str(&mode_str).map_err(|e| JsValue::from_str(&e.to_string()))?
        } else {
            Mode::Normal
        };

        let dict = dictionary.inner;
        let user_dict = user_dictionary.map(|d| d.inner);

        let segmenter = Segmenter::new(m, dict, user_dict);
        let inner = LinderaTokenizer::new(segmenter);

        Ok(Tokenizer { inner })
    }

    /// Tokenizes the input text.
    pub fn tokenize(&self, input_text: &str) -> Result<Vec<JsToken>, JsValue> {
        let tokens = self
            .inner
            .tokenize(input_text)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let js_tokens = tokens.into_iter().map(JsToken::from_token).collect();

        Ok(js_tokens)
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

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("関西国際空港限定トートバッグ").unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].surface, "関西国際空港");
        assert_eq!(tokens[1].surface, "限定");
        assert_eq!(tokens[2].surface, "トートバッグ");
        assert_eq!(tokens[0].get_detail(0), Some("名詞".to_string()));
    }
}
