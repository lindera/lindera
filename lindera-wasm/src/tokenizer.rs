use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;
use wasm_bindgen::prelude::*;

use lindera_binding_core::{CoreTokenizer, CoreTokenizerBuilder};

use crate::dictionary::{JsDictionary, JsUserDictionary};
use crate::token::token_view_to_js;

/// Parses optional filter arguments (a JS value) into a JSON value.
fn parse_filter_args(args: JsValue) -> Result<Value, JsValue> {
    if args.is_undefined() || args.is_null() {
        Ok(Value::Object(serde_json::Map::new()))
    } else {
        serde_wasm_bindgen::from_value::<Value>(args).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Mutable builder configuration shared between chained builder handles.
struct BuilderState {
    /// The backing binding-core builder (used for URI-based dictionary loading).
    inner: CoreTokenizerBuilder,
    /// Pre-loaded dictionary instance, used instead of URI-based loading.
    dictionary_instance: Option<JsDictionary>,
    /// Pre-loaded user dictionary instance, used instead of URI-based loading.
    user_dictionary_instance: Option<JsUserDictionary>,
    /// Mode string stored for use when building with a dictionary instance.
    mode_for_instance: Option<String>,
}

/// Builder for creating a [`Tokenizer`] instance.
///
/// `TokenizerBuilder` provides a fluent API for configuring and building a tokenizer
/// with various options such as dictionary selection, tokenization mode, character filters,
/// and token filters. The build-flow orchestration is delegated to
/// [`lindera_binding_core::CoreTokenizerBuilder`].
///
/// Setters return a builder handle sharing the same configuration, so both the
/// chained style (`builder.setMode(...).setDictionary(...).build()`) and the
/// sequential style (one call per statement) work. wasm-bindgen cannot return
/// the borrowed JS `this`, hence the shared-state handle instead.
#[wasm_bindgen]
pub struct TokenizerBuilder {
    /// Configuration shared by every handle returned from the setters.
    state: Rc<RefCell<BuilderState>>,
}

#[wasm_bindgen]
impl TokenizerBuilder {
    /// Creates a new `TokenizerBuilder` instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let inner = CoreTokenizerBuilder::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self {
            state: Rc::new(RefCell::new(BuilderState {
                inner,
                dictionary_instance: None,
                user_dictionary_instance: None,
                mode_for_instance: None,
            })),
        })
    }

    /// Builds and returns a configured [`Tokenizer`] instance.
    ///
    /// If a dictionary instance was set via `setDictionaryInstance()`,
    /// it will be used directly instead of loading from a URI.
    ///
    /// The builder remains usable afterwards, so multiple tokenizers can be
    /// built from the same configuration.
    pub fn build(&self) -> Result<Tokenizer, JsValue> {
        let state = self.state.borrow();
        if let Some(dict) = state.dictionary_instance.clone() {
            // Build tokenizer using the pre-loaded dictionary instance
            // (dictionaries are cheap to clone: their payload is shared).
            let user_dict = state.user_dictionary_instance.clone().map(|d| d.inner);
            let inner = CoreTokenizer::from_segmenter(
                state.mode_for_instance.as_deref().unwrap_or("normal"),
                dict.inner,
                user_dict,
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

            Ok(Tokenizer { inner })
        } else {
            let inner = state
                .inner
                .build()
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            Ok(Tokenizer { inner })
        }
    }

    /// Returns a new handle sharing this builder's configuration.
    fn share(&self) -> TokenizerBuilder {
        TokenizerBuilder {
            state: Rc::clone(&self.state),
        }
    }

    /// Sets the tokenization mode.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setMode")]
    pub fn set_mode(&self, mode: &str) -> Result<TokenizerBuilder, JsValue> {
        {
            let mut state = self.state.borrow_mut();
            state
                .inner
                .set_mode(mode)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            state.mode_for_instance = Some(mode.to_string());
        }

        Ok(self.share())
    }

    /// Sets the dictionary to use for tokenization by URI.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setDictionary")]
    pub fn set_dictionary(&self, uri: &str) -> TokenizerBuilder {
        {
            let mut state = self.state.borrow_mut();
            state.inner.set_dictionary(uri);
            state.dictionary_instance = None;
        }

        self.share()
    }

    /// Sets a pre-loaded dictionary instance for tokenization.
    ///
    /// Use this method when the dictionary has been loaded from bytes
    /// (e.g., via `loadDictionaryFromBytes()`) instead of from a URI.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setDictionaryInstance")]
    pub fn set_dictionary_instance(&self, dictionary: JsDictionary) -> TokenizerBuilder {
        self.state.borrow_mut().dictionary_instance = Some(dictionary);

        self.share()
    }

    /// Sets a pre-loaded user dictionary instance.
    ///
    /// Use this method with a user dictionary loaded from bytes via
    /// `loadUserDictionaryFromBytes()` or `loadUserDictionaryBinFromBytes()`;
    /// URI-based user dictionaries are not available on WebAssembly (#972).
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setUserDictionaryInstance")]
    pub fn set_user_dictionary_instance(&self, user_dictionary: JsUserDictionary) -> TokenizerBuilder {
        self.state.borrow_mut().user_dictionary_instance = Some(user_dictionary);

        self.share()
    }

    /// Sets whether to keep whitespace tokens in the output.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setKeepWhitespace")]
    pub fn set_keep_whitespace(&self, keep: bool) -> TokenizerBuilder {
        self.state.borrow_mut().inner.set_keep_whitespace(keep);

        self.share()
    }

    /// Appends a character filter to the tokenization pipeline.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "appendCharacterFilter")]
    pub fn append_character_filter(&self, name: &str, args: JsValue) -> Result<TokenizerBuilder, JsValue> {
        let a = parse_filter_args(args)?;
        self.state.borrow_mut().inner.append_character_filter(name, &a);

        Ok(self.share())
    }

    /// Appends a token filter to the tokenization pipeline.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "appendTokenFilter")]
    pub fn append_token_filter(&self, name: &str, args: JsValue) -> Result<TokenizerBuilder, JsValue> {
        let a = parse_filter_args(args)?;
        self.state.borrow_mut().inner.append_token_filter(name, &a);

        Ok(self.share())
    }

    // Python-style aliases (snake_case)

    /// Sets the tokenization mode (snake_case alias).
    #[wasm_bindgen(js_name = "set_mode")]
    pub fn py_set_mode(&self, mode: &str) -> Result<TokenizerBuilder, JsValue> {
        self.set_mode(mode)
    }

    /// Sets the dictionary by URI (snake_case alias).
    #[wasm_bindgen(js_name = "set_dictionary")]
    pub fn py_set_dictionary(&self, uri: &str) -> TokenizerBuilder {
        self.set_dictionary(uri)
    }

    /// Sets a pre-loaded dictionary instance (snake_case alias).
    #[wasm_bindgen(js_name = "set_dictionary_instance")]
    pub fn py_set_dictionary_instance(&self, dictionary: JsDictionary) -> TokenizerBuilder {
        self.set_dictionary_instance(dictionary)
    }

    /// Sets a pre-loaded user dictionary instance (snake_case alias).
    #[wasm_bindgen(js_name = "set_user_dictionary_instance")]
    pub fn py_set_user_dictionary_instance(
        &self,
        user_dictionary: JsUserDictionary,
    ) -> TokenizerBuilder {
        self.set_user_dictionary_instance(user_dictionary)
    }

    /// Sets whether to keep whitespace tokens (snake_case alias).
    #[wasm_bindgen(js_name = "set_keep_whitespace")]
    pub fn py_set_keep_whitespace(&self, keep: bool) -> TokenizerBuilder {
        self.set_keep_whitespace(keep)
    }

    /// Appends a character filter (snake_case alias).
    #[wasm_bindgen(js_name = "append_character_filter")]
    pub fn py_append_character_filter(
        &self,
        name: &str,
        args: JsValue,
    ) -> Result<TokenizerBuilder, JsValue> {
        self.append_character_filter(name, args)
    }

    /// Appends a token filter (snake_case alias).
    #[wasm_bindgen(js_name = "append_token_filter")]
    pub fn py_append_token_filter(
        &self,
        name: &str,
        args: JsValue,
    ) -> Result<TokenizerBuilder, JsValue> {
        self.append_token_filter(name, args)
    }
}

/// A tokenizer for morphological analysis.
#[wasm_bindgen]
pub struct Tokenizer {
    /// The backing binding-core tokenizer.
    inner: CoreTokenizer,
}

#[wasm_bindgen]
impl Tokenizer {
    #[wasm_bindgen(constructor)]
    pub fn new(
        dictionary: JsDictionary,
        mode: Option<String>,
        user_dictionary: Option<JsUserDictionary>,
    ) -> Result<Tokenizer, JsValue> {
        let user_dict = user_dictionary.map(|d| d.inner);
        let inner = CoreTokenizer::from_segmenter(
            mode.as_deref().unwrap_or("normal"),
            dictionary.inner,
            user_dict,
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Tokenizer { inner })
    }

    /// Tokenizes the input text.
    ///
    /// Tokens are returned as plain JS objects with camelCase fields, so
    /// they carry no Rust-side allocation for the caller to release and
    /// serialize without conversion.
    pub fn tokenize(&self, input_text: &str) -> Result<Vec<JsValue>, JsValue> {
        let views = self
            .inner
            .tokenize(input_text)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(views.into_iter().map(token_view_to_js).collect())
    }

    /// Tokenizes the input text and returns only the token surfaces.
    ///
    /// This is the fast path for wakati-style use: no Token objects are
    /// created and no morphological details are loaded, so it is
    /// significantly faster than `tokenize` when only the surface strings
    /// are needed. The surfaces equal `tokenize(text).map((t) => t.surface)`.
    /// Unrelated to Web Workers.
    #[wasm_bindgen(js_name = "tokenizeSurfaces")]
    pub fn tokenize_surfaces(&self, input_text: &str) -> Result<Vec<String>, JsValue> {
        let views = self
            .inner
            .tokenize_surfaces(input_text)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(views.into_iter().map(|view| view.surface).collect())
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
        for (views, cost) in results {
            let entry = js_sys::Object::new();
            let inner = js_sys::Array::new();
            for view in views {
                inner.push(&token_view_to_js(view));
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

    /// Tokenizes the input text and returns only the token surfaces (snake_case alias).
    #[wasm_bindgen(js_name = "tokenize_surfaces")]
    pub fn py_tokenize_surfaces(&self, input_text: &str) -> Result<Vec<String>, JsValue> {
        self.tokenize_surfaces(input_text)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Reads a string field from a plain-object token (#930: tokens are
    /// plain JS objects with camelCase keys, not class instances).
    #[cfg(target_arch = "wasm32")]
    fn token_str(token: &wasm_bindgen::JsValue, key: &str) -> String {
        js_sys::Reflect::get(token, &key.into())
            .unwrap()
            .as_string()
            .unwrap()
    }

    /// Reads a numeric field from a plain-object token.
    #[cfg(target_arch = "wasm32")]
    fn token_num(token: &wasm_bindgen::JsValue, key: &str) -> f64 {
        js_sys::Reflect::get(token, &key.into())
            .unwrap()
            .as_f64()
            .unwrap()
    }

    /// Reads a boolean field from a plain-object token.
    #[cfg(target_arch = "wasm32")]
    fn token_bool(token: &wasm_bindgen::JsValue, key: &str) -> bool {
        js_sys::Reflect::get(token, &key.into())
            .unwrap()
            .as_bool()
            .unwrap()
    }

    /// Reads the `details` array from a plain-object token.
    #[cfg(target_arch = "wasm32")]
    fn token_details(token: &wasm_bindgen::JsValue) -> Vec<String> {
        let details = js_sys::Reflect::get(token, &"details".into()).unwrap();
        js_sys::Array::from(&details)
            .iter()
            .map(|d| d.as_string().unwrap())
            .collect()
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("関西国際空港限定トートバッグ").unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(token_str(&tokens[0], "surface"), "関西国際空港");
        assert_eq!(token_str(&tokens[1], "surface"), "限定");
        assert_eq!(token_str(&tokens[2], "surface"), "トートバッグ");
        assert_eq!(token_details(&tokens[0])[0], "名詞");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_surfaces_matches_tokenize() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let text = "関西国際空港限定トートバッグ";
        let expected: Vec<String> = tokenizer
            .tokenize(text)
            .unwrap()
            .iter()
            .map(|t| token_str(t, "surface"))
            .collect();
        let surfaces = tokenizer.tokenize_surfaces(text).unwrap();
        assert_eq!(expected, surfaces);
        assert_eq!(surfaces, vec!["関西国際空港", "限定", "トートバッグ"]);

        // Repeated calls on one instance reuse the internal lattice and
        // must stay stable; the snake_case alias must match too.
        for _ in 0..10 {
            assert_eq!(surfaces, tokenizer.tokenize_surfaces(text).unwrap());
        }
        assert_eq!(surfaces, tokenizer.py_tokenize_surfaces(text).unwrap());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_with_ipadic() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("すもももももももものうち").unwrap();

        assert_eq!(tokens.len(), 7);
        assert_eq!(token_str(&tokens[0], "surface"), "すもも");
        assert_eq!(token_str(&tokens[6], "surface"), "うち");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_method_chaining() {
        use crate::TokenizerBuilder;

        // Chained style: every setter returns a handle sharing the same state.
        let tokenizer = TokenizerBuilder::new()
            .unwrap()
            .set_mode("normal")
            .unwrap()
            .set_dictionary("embedded://ipadic")
            .build()
            .unwrap();

        let surfaces = tokenizer.tokenize_surfaces("すもももももももものうち").unwrap();
        assert_eq!(surfaces.len(), 7);
        assert_eq!(surfaces[0], "すもも");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_handles_share_state() {
        use crate::TokenizerBuilder;

        // A setter called on a returned handle must affect the original builder,
        // and the builder must remain usable after `build()`.
        let builder = TokenizerBuilder::new().unwrap();
        let handle = builder.set_mode("normal").unwrap();
        handle.set_dictionary("embedded://ipadic");

        let first = builder.build().unwrap();
        assert!(!first.tokenize_surfaces("日本語").unwrap().is_empty());

        let second = builder.build().unwrap();
        assert_eq!(
            first.tokenize_surfaces("日本語").unwrap(),
            second.tokenize_surfaces("日本語").unwrap()
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_token_properties() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("関西国際空港").unwrap();

        assert_eq!(tokens.len(), 1);

        let token = &tokens[0];
        assert_eq!(token_str(token, "surface"), "関西国際空港");
        assert_eq!(token_num(token, "byteStart"), 0.0);
        assert_eq!(token_num(token, "byteEnd"), "関西国際空港".len() as f64);
        assert_eq!(token_num(token, "position"), 0.0);
        assert!(!token_bool(token, "isUnknown"));
        assert!(!token_details(token).is_empty());
        assert!(token_num(token, "wordId") > 0.0);
    }

    /// #930: details are read by indexing the array, which replaces the
    /// `getDetail(i)` method that existed while tokens were class
    /// instances.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_token_details_by_index() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("東京").unwrap();

        assert!(!tokens.is_empty());

        let details = token_details(&tokens[0]);
        assert!(!details.is_empty());
        assert_eq!(details[0], "名詞");

        // Out-of-range reads yield undefined, as for any JS array.
        let raw = js_sys::Reflect::get(&tokens[0], &"details".into()).unwrap();
        let out_of_range = js_sys::Reflect::get_u32(&raw, 9999).unwrap();
        assert!(out_of_range.is_undefined());
    }

    /// #930: tokens are plain objects, so they serialize without any
    /// conversion step and carry no Rust-side allocation to release.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_token_is_plain_serializable_object() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("東京").unwrap();
        let token = &tokens[0];

        // A plain object, not a wasm-bindgen class instance: its prototype
        // is Object.prototype, i.e. its constructor is Object itself.
        assert!(token.is_object());
        let proto = js_sys::Object::get_prototype_of(token);
        let ctor = js_sys::Reflect::get(&proto, &"constructor".into()).unwrap();
        let ctor_name = js_sys::Reflect::get(&ctor, &"name".into())
            .unwrap()
            .as_string()
            .unwrap();
        assert_eq!(ctor_name, "Object");

        // Round-trips through JSON with its fields intact.
        let json = js_sys::JSON::stringify(token).unwrap();
        let parsed = js_sys::JSON::parse(&String::from(json)).unwrap();
        assert_eq!(token_str(&parsed, "surface"), token_str(token, "surface"));
        assert_eq!(
            token_num(&parsed, "byteStart"),
            token_num(token, "byteStart")
        );
        assert_eq!(token_details(&parsed), token_details(token));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_decompose_mode() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("decompose").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("関西国際空港").unwrap();

        assert!(!tokens.is_empty());

        let reconstructed: String = tokens.iter().map(|t| token_str(t, "surface")).collect();
        assert_eq!(reconstructed, "関西国際空港");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_empty_string() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("").unwrap();

        assert!(tokens.is_empty());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_nbest() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let results = tokenizer
            .tokenize_nbest("すもももももももものうち", 3, None, None)
            .unwrap();

        assert!(!results.is_null());
        assert!(!results.is_undefined());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_set_mode_invalid() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
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
        assert_eq!(token_str(&tokens[0], "surface"), "東京");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_with_dictionary_instance() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;

        let dict = load_dictionary("embedded://ipadic").unwrap();

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("normal").unwrap();
        builder.set_dictionary_instance(dict);

        let tokenizer = builder.build().unwrap();
        let tokens = tokenizer.tokenize("東京タワー").unwrap();

        assert!(!tokens.is_empty());
        assert_eq!(token_str(&tokens[0], "surface"), "東京");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_with_dictionary_instance_decompose_mode() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;

        let dict = load_dictionary("embedded://ipadic").unwrap();

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode("decompose").unwrap();
        builder.set_dictionary_instance(dict);

        let tokenizer = builder.build().unwrap();
        let tokens = tokenizer.tokenize("関西国際空港").unwrap();

        assert!(!tokens.is_empty());
        let reconstructed: String = tokens.iter().map(|t| token_str(t, "surface")).collect();
        assert_eq!(reconstructed, "関西国際空港");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_builder_with_dictionary_instance_default_mode() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;

        let dict = load_dictionary("embedded://ipadic").unwrap();

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary_instance(dict);

        let tokenizer = builder.build().unwrap();
        let tokens = tokenizer.tokenize("すもも").unwrap();

        assert!(!tokens.is_empty());
    }
}
