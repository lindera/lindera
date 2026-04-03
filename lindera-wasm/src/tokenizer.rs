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
    /// Pre-loaded dictionary instance, used instead of URI-based loading.
    dictionary_instance: Option<JsDictionary>,
    /// Pre-loaded user dictionary instance, used instead of URI-based loading.
    user_dictionary_instance: Option<JsUserDictionary>,
    /// Mode string stored for use when building with a dictionary instance.
    mode_for_instance: Option<String>,
}

#[wasm_bindgen]
impl TokenizerBuilder {
    /// Creates a new `TokenizerBuilder` instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let inner =
            LinderaTokenizerBuilder::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self {
            inner,
            dictionary_instance: None,
            user_dictionary_instance: None,
            mode_for_instance: None,
        })
    }

    /// Builds and returns a configured [`Tokenizer`] instance.
    ///
    /// If a dictionary instance was set via `setDictionaryInstance()`,
    /// it will be used directly instead of loading from a URI.
    pub fn build(self) -> Result<Tokenizer, JsValue> {
        if let Some(dict) = self.dictionary_instance {
            // Build tokenizer using the pre-loaded dictionary instance
            let m = self
                .mode_for_instance
                .as_deref()
                .map(Mode::from_str)
                .transpose()
                .map_err(|e| JsValue::from_str(&e.to_string()))?
                .unwrap_or(Mode::Normal);

            let user_dict = self.user_dictionary_instance.map(|d| d.inner);

            let segmenter = Segmenter::new(m, dict.inner, user_dict);
            let inner = LinderaTokenizer::new(segmenter);

            Ok(Tokenizer { inner })
        } else {
            let inner = self
                .inner
                .build()
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            Ok(Tokenizer { inner })
        }
    }

    /// Sets the tokenization mode.
    #[wasm_bindgen(js_name = "setMode")]
    pub fn set_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        let m = Mode::from_str(mode).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.inner.set_segmenter_mode(&m);
        self.mode_for_instance = Some(mode.to_string());

        Ok(())
    }

    /// Sets the dictionary to use for tokenization by URI.
    #[wasm_bindgen(js_name = "setDictionary")]
    pub fn set_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.inner.set_segmenter_dictionary(uri);
        self.dictionary_instance = None;

        Ok(())
    }

    /// Sets a pre-loaded dictionary instance for tokenization.
    ///
    /// Use this method when the dictionary has been loaded from bytes
    /// (e.g., via `loadDictionaryFromBytes()`) instead of from a URI.
    #[wasm_bindgen(js_name = "setDictionaryInstance")]
    pub fn set_dictionary_instance(&mut self, dictionary: JsDictionary) {
        self.dictionary_instance = Some(dictionary);
    }

    /// Sets a user-defined dictionary by URI.
    #[wasm_bindgen(js_name = "setUserDictionary")]
    pub fn set_user_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.inner.set_segmenter_user_dictionary(uri);
        self.user_dictionary_instance = None;

        Ok(())
    }

    /// Sets a pre-loaded user dictionary instance.
    ///
    /// Use this method when the user dictionary has been loaded from bytes
    /// instead of from a URI.
    #[wasm_bindgen(js_name = "setUserDictionaryInstance")]
    pub fn set_user_dictionary_instance(&mut self, user_dictionary: JsUserDictionary) {
        self.user_dictionary_instance = Some(user_dictionary);
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

    #[wasm_bindgen(js_name = "set_dictionary_instance")]
    pub fn py_set_dictionary_instance(&mut self, dictionary: JsDictionary) {
        self.set_dictionary_instance(dictionary)
    }

    #[wasm_bindgen(js_name = "set_user_dictionary")]
    pub fn py_set_user_dictionary(&mut self, uri: &str) -> Result<(), JsValue> {
        self.set_user_dictionary(uri)
    }

    #[wasm_bindgen(js_name = "set_user_dictionary_instance")]
    pub fn py_set_user_dictionary_instance(&mut self, user_dictionary: JsUserDictionary) {
        self.set_user_dictionary_instance(user_dictionary)
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

    /// Tokenizes the input text and returns N-best results.
    ///
    /// Returns an array of arrays, where each inner array contains Token JSON objects.
    #[wasm_bindgen(js_name = "tokenizeNbest")]
    pub fn tokenize_nbest(
        &self,
        input_text: &str,
        n: usize,
        unique: Option<bool>,
        cost_threshold: Option<i64>,
    ) -> Result<JsValue, JsValue> {
        let results = self
            .inner
            .tokenize_nbest(input_text, n, unique.unwrap_or(false), cost_threshold)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let outer = js_sys::Array::new();
        for (tokens, cost) in results {
            let entry = js_sys::Object::new();
            let inner = js_sys::Array::new();
            for token in tokens {
                let js_token = JsToken::from_token(token);
                inner.push(&js_token.to_json());
            }
            js_sys::Reflect::set(&entry, &"tokens".into(), &inner).unwrap();
            js_sys::Reflect::set(&entry, &"cost".into(), &JsValue::from(cost as f64)).unwrap();
            outer.push(&entry);
        }

        Ok(outer.into())
    }

    /// Tokenizes the input text and returns N-best results (snake_case alias).
    #[wasm_bindgen(js_name = "tokenize_nbest")]
    pub fn py_tokenize_nbest(
        &self,
        input_text: &str,
        n: usize,
        unique: Option<bool>,
        cost_threshold: Option<i64>,
    ) -> Result<JsValue, JsValue> {
        self.tokenize_nbest(input_text, n, unique, cost_threshold)
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

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_with_ipadic() {
        use crate::TokenizerBuilder;

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("すもももももももものうち").unwrap();

        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0].surface, "すもも");
        assert_eq!(tokens[1].surface, "も");
        assert_eq!(tokens[2].surface, "もも");
        assert_eq!(tokens[3].surface, "も");
        assert_eq!(tokens[4].surface, "もも");
        assert_eq!(tokens[5].surface, "の");
        assert_eq!(tokens[6].surface, "うち");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_token_properties() {
        use crate::TokenizerBuilder;

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("関西国際空港").unwrap();

        assert_eq!(tokens.len(), 1);

        let token = &tokens[0];
        assert_eq!(token.surface, "関西国際空港");
        assert_eq!(token.byte_start, 0);
        assert_eq!(token.byte_end, "関西国際空港".len());
        assert_eq!(token.position, 0);
        assert!(!token.is_unknown);
        assert!(!token.details.is_empty());
        assert!(token.word_id > 0);
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_token_get_detail() {
        use crate::TokenizerBuilder;

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("東京").unwrap();

        assert!(!tokens.is_empty());

        let token = &tokens[0];

        // Valid index returns Some
        let first_detail = token.get_detail(0);
        assert!(first_detail.is_some());
        assert_eq!(first_detail.unwrap(), token.details[0]);

        // Out of bounds returns None
        assert!(token.get_detail(9999).is_none());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_token_to_json() {
        use crate::TokenizerBuilder;

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("東京").unwrap();

        let json = tokens[0].to_json();

        // to_json should return a non-null JsValue
        assert!(!json.is_null());
        assert!(!json.is_undefined());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_decompose_mode() {
        use crate::TokenizerBuilder;

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("decompose").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("関西国際空港").unwrap();

        // In decompose mode, compound words may be split further
        assert!(!tokens.is_empty());

        // Verify all surfaces concatenated form the original text
        let reconstructed: String = tokens.iter().map(|t| t.surface.as_str()).collect();
        assert_eq!(reconstructed, "関西国際空港");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_empty_string() {
        use crate::TokenizerBuilder;

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("").unwrap();

        assert!(tokens.is_empty());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_nbest() {
        use crate::TokenizerBuilder;

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic").unwrap();

        let tokenizer = builder.build().unwrap();

        let results = tokenizer
            .tokenize_nbest("すもももももももものうち", 3, None, None)
            .unwrap();

        // Should return a JsValue (array of results)
        assert!(!results.is_null());
        assert!(!results.is_undefined());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_set_mode_invalid() {
        use crate::TokenizerBuilder;

        let mut builder = TokenizerBuilder::new().unwrap();
        let result = builder.set_mode("invalid_mode");

        assert!(result.is_err());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenizer_with_dictionary_constructor() {
        use crate::Tokenizer;
        use crate::dictionary::load_dictionary;

        let dict = load_dictionary("embedded://ipadic").unwrap();
        let tokenizer = Tokenizer::new(dict, Some("normal".to_string()), None).unwrap();

        let tokens = tokenizer.tokenize("東京タワー").unwrap();

        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].surface, "東京");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_with_dictionary_instance() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;

        let dict = load_dictionary("embedded://ipadic").unwrap();

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary_instance(dict);

        let tokenizer = builder.build().unwrap();
        let tokens = tokenizer.tokenize("東京タワー").unwrap();

        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].surface, "東京");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_with_dictionary_instance_decompose_mode() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;

        let dict = load_dictionary("embedded://ipadic").unwrap();

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("decompose").unwrap();
        builder.set_dictionary_instance(dict);

        let tokenizer = builder.build().unwrap();
        let tokens = tokenizer.tokenize("関西国際空港").unwrap();

        assert!(!tokens.is_empty());
        let reconstructed: String = tokens.iter().map(|t| t.surface.as_str()).collect();
        assert_eq!(reconstructed, "関西国際空港");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_with_dictionary_instance_default_mode() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;

        // Set dictionary instance without calling set_mode (should default to Normal)
        let dict = load_dictionary("embedded://ipadic").unwrap();

        let mut builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary_instance(dict);

        let tokenizer = builder.build().unwrap();
        let tokens = tokenizer.tokenize("すもも").unwrap();

        assert!(!tokens.is_empty());
    }
}
