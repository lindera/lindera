use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;
use wasm_bindgen::prelude::*;

use lindera_binding_core::{CoreTokenizer, CoreTokenizerBuilder};

use crate::dictionary::{JsDictionary, JsUserDictionary};
use crate::token::token_view_to_js;

/// Converts a JS value into a JSON value.
///
/// # Arguments
///
/// * `value` - The JS value; `null` and `undefined` both become JSON `null`.
///
/// # Returns
///
/// The JSON value, or the conversion error when the value has no JSON form.
fn js_to_json(value: JsValue) -> Result<Value, serde_wasm_bindgen::Error> {
    if value.is_undefined() || value.is_null() {
        Ok(Value::Null)
    } else {
        serde_wasm_bindgen::from_value::<Value>(value)
    }
}

/// Parses optional filter arguments (a JS value) into a JSON value.
///
/// # Arguments
///
/// * `args` - The filter arguments; `null` or `undefined` mean no arguments.
///
/// # Returns
///
/// The arguments as JSON (an empty object when none were given), or an error
/// string when they have no JSON form.
fn parse_filter_args(args: JsValue) -> Result<Value, JsValue> {
    match js_to_json(args).map_err(|e| JsValue::from_str(&e.to_string()))? {
        Value::Null => Ok(Value::Object(serde_json::Map::new())),
        value => Ok(value),
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
    /// The value last passed to `setSpacePenalty()` (JSON `null` when unset),
    /// applied to a dictionary instance at build time; the URI path gets it
    /// through `inner`.
    space_penalty: Value,
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
                space_penalty: Value::Null,
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
    ///
    /// The `setSpacePenalty()` setting applies to a dictionary instance as it
    /// does to a dictionary set by URI.
    pub fn build(&self) -> Result<Tokenizer, JsValue> {
        let state = self.state.borrow();
        if let Some(dict) = state.dictionary_instance.clone() {
            // Build tokenizer using the pre-loaded dictionary instance
            // (dictionaries are cheap to clone: their payload is shared).
            let user_dict = state.user_dictionary_instance.clone().map(|d| d.inner);
            let inner = CoreTokenizer::from_segmenter_with_space_penalty(
                state.mode_for_instance.as_deref().unwrap_or("normal"),
                dict.inner,
                user_dict,
                &state.space_penalty,
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
    pub fn set_user_dictionary_instance(
        &self,
        user_dictionary: JsUserDictionary,
    ) -> TokenizerBuilder {
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

    /// Sets the left-space penalty (Korean; mecab-ko's
    /// `left-space-penalty-factor`), with the same meaning as
    /// `segmenter.space_penalty` in a YAML config.
    ///
    /// Since v6.1.0 a dictionary that ships rules in its `metadata.json`
    /// (ko-dic does) applies them by default; pass `false` to turn them off.
    ///
    /// Applies both to a dictionary set with `setDictionary()` and to one set
    /// with `setDictionaryInstance()` (e.g. loaded with
    /// `loadDictionaryFromBytes()`).
    ///
    /// # Arguments
    ///
    /// * `value` - One of:
    ///   - `null` / `undefined`: the default, i.e. the rules the dictionary
    ///     ships, if any;
    ///   - `false`: turn the penalty off;
    ///   - `true`: require the dictionary's rules (`build()` fails if it
    ///     ships none);
    ///   - an object such as `{ rules: [{ pos: ["JKS"], cost: 6000 }] }`:
    ///     apply these rules instead.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, enabling method chaining,
    /// or an error string when `value` is not one of the forms above.
    #[wasm_bindgen(js_name = "setSpacePenalty")]
    pub fn set_space_penalty(&self, value: JsValue) -> Result<TokenizerBuilder, JsValue> {
        let value = js_to_json(value)
            .map_err(|e| JsValue::from_str(&format!("invalid space_penalty: {e}")))?;
        {
            let mut state = self.state.borrow_mut();
            state
                .inner
                .set_space_penalty(&value)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            state.space_penalty = value;
        }

        Ok(self.share())
    }

    /// Appends a character filter to the tokenization pipeline.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "appendCharacterFilter")]
    pub fn append_character_filter(
        &self,
        name: &str,
        args: JsValue,
    ) -> Result<TokenizerBuilder, JsValue> {
        let a = parse_filter_args(args)?;
        self.state
            .borrow_mut()
            .inner
            .append_character_filter(name, &a);

        Ok(self.share())
    }

    /// Appends a token filter to the tokenization pipeline.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "appendTokenFilter")]
    pub fn append_token_filter(
        &self,
        name: &str,
        args: JsValue,
    ) -> Result<TokenizerBuilder, JsValue> {
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

    /// Sets the left-space penalty (snake_case alias of `setSpacePenalty`).
    ///
    /// # Arguments
    ///
    /// * `value` - `null` / `undefined`, a boolean, or an object with `rules`.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, or an error string when
    /// `value` is invalid.
    #[wasm_bindgen(js_name = "set_space_penalty")]
    pub fn py_set_space_penalty(&self, value: JsValue) -> Result<TokenizerBuilder, JsValue> {
        self.set_space_penalty(value)
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

        let surfaces = tokenizer
            .tokenize_surfaces("すもももももももものうち")
            .unwrap();
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

    /// Parses a JSON literal into a JS value.
    #[cfg(target_arch = "wasm32")]
    fn js_json(text: &str) -> wasm_bindgen::JsValue {
        js_sys::JSON::parse(text).unwrap()
    }

    /// The explicit rules object used by the space-penalty tests.
    #[cfg(target_arch = "wasm32")]
    fn space_penalty_rules() -> wasm_bindgen::JsValue {
        js_json(r#"{"rules":[{"pos":["JKS"],"cost":6000}]}"#)
    }

    /// Spaced Japanese text whose particle and auxiliary verb IPADIC reads
    /// differently once a left-space penalty applies to them.
    #[cfg(target_arch = "wasm32")]
    const SPACED_TEXT: &str = "私 は 猫 です";

    /// IPADIC-tag rules that penalize particles and auxiliary verbs after a
    /// space, as a JS object for `setSpacePenalty()`.
    #[cfg(target_arch = "wasm32")]
    fn ipadic_rules() -> wasm_bindgen::JsValue {
        js_json(r#"{"rules":[{"pos":["助詞","助動詞"],"cost":100000}]}"#)
    }

    /// The same rules as [`ipadic_rules`], as a Rust config.
    #[cfg(target_arch = "wasm32")]
    fn ipadic_rules_config() -> lindera::space_penalty::SpacePenaltyConfig {
        use lindera::space_penalty::{SpacePenaltyConfig, SpacePenaltyRule};

        SpacePenaltyConfig::new(vec![SpacePenaltyRule::new(["助詞", "助動詞"], 100000)])
    }

    /// Tokenizes `text` into `(surface, part of speech)` pairs.
    #[cfg(target_arch = "wasm32")]
    fn surfaces_and_pos(tokenizer: &crate::Tokenizer, text: &str) -> Vec<(String, String)> {
        tokenizer
            .tokenize(text)
            .unwrap()
            .iter()
            .map(|t| (token_str(t, "surface"), token_details(t)[0].clone()))
            .collect()
    }

    /// Builds `builder` and tokenizes [`SPACED_TEXT`].
    #[cfg(target_arch = "wasm32")]
    fn build_and_tokenize(builder: &crate::TokenizerBuilder) -> Vec<(String, String)> {
        surfaces_and_pos(&builder.build().unwrap(), SPACED_TEXT)
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_set_space_penalty_accepts_the_config_forms() {
        use crate::TokenizerBuilder;
        use wasm_bindgen::JsValue;

        let builder = TokenizerBuilder::new().unwrap();
        for value in [
            JsValue::NULL,
            JsValue::UNDEFINED,
            JsValue::TRUE,
            JsValue::FALSE,
            space_penalty_rules(),
            js_json(r#"{"rules":[]}"#),
        ] {
            let shown = format!("{value:?}");
            assert!(builder.set_space_penalty(value).is_ok(), "rejected {shown}");
        }
        // The snake_case alias and chaining work as for the other setters.
        builder
            .py_set_space_penalty(JsValue::FALSE)
            .unwrap()
            .set_dictionary("embedded://ipadic")
            .build()
            .unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_set_space_penalty_rejects_other_values() {
        use crate::TokenizerBuilder;
        use wasm_bindgen::JsValue;

        let builder = TokenizerBuilder::new().unwrap();
        for value in [
            JsValue::from(1),
            JsValue::from_str("false"),
            js_json(r#"["JKS"]"#),
            js_json(r#"{"rules":"JKS"}"#),
            js_json(r#"{"rules":[{"pos":["JKS"]}]}"#),
            // Values with no JSON form at all.
            js_sys::Uint8Array::new_with_length(2).into(),
            js_sys::Function::new_no_args("").into(),
        ] {
            let shown = format!("{value:?}");
            let message = match builder.set_space_penalty(value) {
                Ok(_) => panic!("accepted {shown}"),
                Err(err) => err.as_string().unwrap(),
            };
            assert!(message.contains("space_penalty"), "{shown}: {message}");
        }
    }

    /// IPADIC ships no left-space penalty rules: requiring them (`true`)
    /// makes `build()` fail, while the other forms build fine, with the
    /// dictionary set by URI or as an instance.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_space_penalty_true_fails_on_ipadic() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;
        use wasm_bindgen::JsValue;

        for by_instance in [false, true] {
            for (value, should_build) in [
                (JsValue::NULL, true),
                (JsValue::FALSE, true),
                (space_penalty_rules(), true),
                (JsValue::TRUE, false),
            ] {
                let shown = format!("{value:?} (instance: {by_instance})");
                let builder = TokenizerBuilder::new().unwrap();
                if by_instance {
                    builder.set_dictionary_instance(load_dictionary("embedded://ipadic").unwrap());
                } else {
                    builder.set_dictionary("embedded://ipadic");
                }
                builder.set_space_penalty(value).unwrap();
                let result = builder.build();
                assert_eq!(result.is_ok(), should_build, "value {shown}");
                if let Err(err) = result {
                    let message = err.as_string().unwrap();
                    assert!(message.contains("space_penalty"), "{shown}: {message}");
                }
            }
        }
    }

    /// Rules change the tokenization of a dictionary set by URI, and
    /// `false` or `null` bring the default output back on the same builder.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_space_penalty_changes_tokenization() {
        use crate::TokenizerBuilder;
        use wasm_bindgen::JsValue;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary("embedded://ipadic");
        let baseline = build_and_tokenize(&builder);
        assert!(
            baseline.contains(&("は".to_string(), "助詞".to_string())),
            "fixture assumption: {baseline:?}"
        );

        builder.set_space_penalty(ipadic_rules()).unwrap();
        let penalized = build_and_tokenize(&builder);
        assert_ne!(penalized, baseline);
        assert!(
            !penalized.contains(&("は".to_string(), "助詞".to_string())),
            "{penalized:?}"
        );

        builder.set_space_penalty(JsValue::FALSE).unwrap();
        assert_eq!(build_and_tokenize(&builder), baseline);

        builder.set_space_penalty(ipadic_rules()).unwrap();
        assert_eq!(build_and_tokenize(&builder), penalized);
        builder.set_space_penalty(JsValue::NULL).unwrap();
        assert_eq!(build_and_tokenize(&builder), baseline);
    }

    /// A dictionary instance that ships rules (as a ko-dic loaded with
    /// `loadDictionaryFromBytes()` does) applies them by default, and every
    /// `setSpacePenalty()` form takes effect on it, without changing the
    /// caller's dictionary object.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_space_penalty_applies_to_a_dictionary_instance() {
        use std::sync::Arc;

        use crate::dictionary::load_dictionary;
        use crate::{Tokenizer, TokenizerBuilder};
        use wasm_bindgen::JsValue;

        let plain = load_dictionary("embedded://ipadic").unwrap();
        let mut shipping = plain.clone();
        Arc::make_mut(&mut shipping.inner.metadata).space_penalty = Some(ipadic_rules_config());

        // The constructor keeps the dictionary default.
        let baseline = surfaces_and_pos(
            &Tokenizer::new(plain.clone(), None, None).unwrap(),
            SPACED_TEXT,
        );
        let penalized = surfaces_and_pos(
            &Tokenizer::new(shipping.clone(), None, None).unwrap(),
            SPACED_TEXT,
        );
        assert_ne!(penalized, baseline);

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary_instance(shipping.clone());
        for (value, expected) in [
            (JsValue::NULL, &penalized),
            (JsValue::FALSE, &baseline),
            (JsValue::TRUE, &penalized),
            (js_json(r#"{"rules":[]}"#), &baseline),
            (JsValue::UNDEFINED, &penalized),
        ] {
            let shown = format!("{value:?}");
            builder.set_space_penalty(value).unwrap();
            assert_eq!(&build_and_tokenize(&builder), expected, "value {shown}");
        }
        // The builder changed only its tokenizer's copy of the metadata.
        assert_eq!(
            shipping.inner.metadata.space_penalty,
            Some(ipadic_rules_config())
        );

        // Explicit rules apply to a dictionary that ships none.
        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary_instance(plain);
        builder.set_space_penalty(ipadic_rules()).unwrap();
        assert_eq!(build_and_tokenize(&builder), penalized);
    }

    /// Rules that cannot be applied to a dictionary instance fail the build,
    /// as they do for a dictionary set by URI, instead of being dropped with
    /// a warning by the segmenter.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_space_penalty_rules_that_cannot_apply_fail_the_build() {
        use std::sync::Arc;

        use lindera_dictionary::dictionary::schema::Schema;

        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;
        use wasm_bindgen::JsValue;

        // More rules than the penalty table can index, on both paths.
        let rule = r#"{"pos":["助詞"],"cost":1}"#;
        let too_many = js_json(&format!(r#"{{"rules":[{}]}}"#, vec![rule; 256].join(",")));
        for by_instance in [false, true] {
            let builder = TokenizerBuilder::new().unwrap();
            if by_instance {
                builder.set_dictionary_instance(load_dictionary("embedded://ipadic").unwrap());
            } else {
                builder.set_dictionary("embedded://ipadic");
            }
            builder.set_space_penalty(too_many.clone()).unwrap();
            let message = match builder.build() {
                Ok(_) => panic!("built with 256 rules (instance: {by_instance})"),
                Err(err) => err.as_string().unwrap(),
            };
            assert!(message.contains("at most 255 rules"), "{message}");
        }

        // A schema without a part-of-speech field.
        let mut dict = load_dictionary("embedded://ipadic").unwrap();
        Arc::make_mut(&mut dict.inner.metadata).dictionary_schema = Schema::new(
            [
                "surface",
                "left_context_id",
                "right_context_id",
                "cost",
                "reading",
            ]
            .map(String::from)
            .to_vec(),
        );
        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary_instance(dict);
        assert!(builder.build().is_ok());

        builder.set_space_penalty(ipadic_rules()).unwrap();
        let message = match builder.build() {
            Ok(_) => panic!("built with rules on a schema without a part-of-speech field"),
            Err(err) => err.as_string().unwrap(),
        };
        assert!(message.contains("part-of-speech"), "{message}");

        builder.set_space_penalty(JsValue::FALSE).unwrap();
        assert!(builder.build().is_ok());
    }
}
