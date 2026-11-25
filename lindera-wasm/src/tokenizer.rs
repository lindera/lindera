use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::rc::Rc;

use serde_json::Value;
use wasm_bindgen::prelude::*;

use lindera_binding::argument::Positive;
use lindera_binding::{CoreTokenizer, CoreTokenizerBuilder, TokenView};

use crate::argument::{integer_arg, optional_i64_from_bigint, optional_integer_arg};
use crate::dictionary::{JsDictionary, JsUserDictionary, user_dictionary_from_js};
use crate::json::json_from_js;
use crate::mode::{JsMode, mode_from_js};
use crate::token::token_view_to_js;

/// Converts a JS value into a JSON value.
///
/// # Arguments
///
/// * `value` - The JS value; `null` and `undefined` both become JSON `null`.
///
/// # Returns
///
/// The JSON value, or the conversion error when the value has no JSON form
/// or nests more than 128 levels deep.
fn js_to_json(value: JsValue) -> Result<Value, serde_wasm_bindgen::Error> {
    if value.is_undefined() || value.is_null() {
        Ok(Value::Null)
    } else {
        json_from_js(value)
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
    /// The backing lindera-binding builder, which holds every setting but the
    /// dictionary instances (the dictionary URI included).
    inner: CoreTokenizerBuilder,
    /// Pre-loaded dictionary instance, used instead of the dictionary URI.
    dictionary_instance: Option<JsDictionary>,
    /// Pre-loaded user dictionary instance.
    user_dictionary_instance: Option<JsUserDictionary>,
}

/// Builder for creating a [`Tokenizer`] instance.
///
/// `TokenizerBuilder` provides a fluent API for configuring and building a tokenizer
/// with various options such as dictionary selection, tokenization mode, character filters,
/// and token filters. The build-flow orchestration is delegated to
/// [`lindera_binding::CoreTokenizerBuilder`].
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
            })),
        })
    }

    /// Builds and returns a configured [`Tokenizer`] instance.
    ///
    /// The dictionary is the one set last, by `setDictionary()` or
    /// `setDictionaryInstance()`, and the user dictionary is the one set by
    /// `setUserDictionaryInstance()`, if any. Every other setting applies
    /// whichever way the dictionary was set.
    ///
    /// The builder remains usable afterwards, so multiple tokenizers can be
    /// built from the same configuration.
    ///
    /// # Returns
    ///
    /// The tokenizer, or an error string when no dictionary was set, the
    /// dictionary cannot be loaded, or a setting or filter is invalid.
    pub fn build(&self) -> Result<Tokenizer, JsValue> {
        let state = self.state.borrow();
        // A dictionary is cheap to clone, as its payload is shared; the user
        // dictionary is copied, as the tokenizer's segmenter owns it.
        let dictionary = state.dictionary_instance.as_ref().map(|d| d.inner.clone());
        let user_dictionary = state
            .user_dictionary_instance
            .as_ref()
            .map(JsUserDictionary::to_lindera);
        let inner = state
            .inner
            .build_with_dictionaries(dictionary, user_dictionary)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Tokenizer { inner })
    }

    /// Returns a new handle sharing this builder's configuration.
    fn share(&self) -> TokenizerBuilder {
        TokenizerBuilder {
            state: Rc::clone(&self.state),
        }
    }

    /// Sets the tokenization mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - The `Mode`, with its penalty for decompose mode. The
    ///   `Mode` stays usable, so the same one can be passed again; the JS
    ///   glue throws for a value that is not a `Mode`.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setMode")]
    pub fn set_mode(&self, mode: &JsMode) -> TokenizerBuilder {
        self.state.borrow_mut().inner.set_mode(mode.as_lindera());
        self.share()
    }

    /// Sets the dictionary to use for tokenization by URI, replacing a
    /// dictionary instance set by `setDictionaryInstance()`.
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
    /// (e.g., via `loadDictionaryFromBytes()`) instead of from a URI. It
    /// replaces a dictionary set by `setDictionary()`, and every other
    /// setting of the builder applies to it as to a dictionary set by URI.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - The dictionary. It is borrowed, so it stays usable
    ///   and can be passed to other tokenizers and builders (#1238).
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setDictionaryInstance")]
    pub fn set_dictionary_instance(&self, dictionary: &JsDictionary) -> TokenizerBuilder {
        self.state.borrow_mut().dictionary_instance = Some(dictionary.clone());

        self.share()
    }

    /// Sets a pre-loaded user dictionary instance.
    ///
    /// Use this method with a user dictionary loaded from bytes via
    /// `loadUserDictionaryFromBytes()` or `loadUserDictionaryBinFromBytes()`;
    /// URI-based user dictionaries are not available on WebAssembly (#972).
    /// It applies to a dictionary set by `setDictionary()` or by
    /// `setDictionaryInstance()`.
    ///
    /// # Arguments
    ///
    /// * `user_dictionary` - The user dictionary. It is borrowed, so it stays
    ///   usable and can be passed to other tokenizers and builders (#1238);
    ///   the builder shares its payload until `build()` copies it.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setUserDictionaryInstance")]
    pub fn set_user_dictionary_instance(
        &self,
        user_dictionary: &JsUserDictionary,
    ) -> TokenizerBuilder {
        self.state.borrow_mut().user_dictionary_instance = Some(user_dictionary.clone());

        self.share()
    }

    /// Sets whether to keep whitespace tokens in the output.
    ///
    /// Kept whitespace also stays in the lattice, so the other tokens can
    /// differ from those of the default output, which skips whitespace as
    /// MeCab does.
    ///
    /// Returns a builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setKeepWhitespace")]
    pub fn set_keep_whitespace(&self, keep: bool) -> TokenizerBuilder {
        self.state.borrow_mut().inner.set_keep_whitespace(keep);

        self.share()
    }

    /// Sets whether whitespace is skipped in the lattice, with the same
    /// meaning as `segmenter.skip_whitespace` in a YAML config.
    ///
    /// `true` skips whitespace, so the words on either side connect directly,
    /// as MeCab does; `false` keeps whitespace in the lattice as a node of
    /// its own, as before v7 (it is still dropped from the output); `null`
    /// or `undefined` goes back to the default of the dictionary's
    /// `metadata.json` (skipping, except with SudachiDict). No effect while
    /// whitespace tokens are kept (`setKeepWhitespace(true)`).
    ///
    /// Applies both to a dictionary set with `setDictionary()` and to one set
    /// with `setDictionaryInstance()`.
    ///
    /// # Arguments
    ///
    /// * `skip` - The setting, or `null` / `undefined` for the dictionary's
    ///   default. The argument is required in the TypeScript declaration, as
    ///   for `setSpacePenalty`.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, enabling method chaining.
    #[wasm_bindgen(js_name = "setSkipWhitespace")]
    pub fn set_skip_whitespace(
        &self,
        #[wasm_bindgen(unchecked_param_type = "boolean | null | undefined")] skip: Option<bool>,
    ) -> TokenizerBuilder {
        self.state.borrow_mut().inner.set_skip_whitespace(skip);

        self.share()
    }

    /// Sets the break characters, with the same meaning as
    /// `segmenter.break_characters` in a YAML config.
    ///
    /// Each character of the string is a break character: no unknown word,
    /// nor, in decompose mode, any system-dictionary word, crosses a
    /// boundary between a break character and another character. An empty
    /// string turns the rule off, which gives MeCab's segmentation; `null`
    /// or `undefined` goes back to the default, `・`.
    ///
    /// # Arguments
    ///
    /// * `chars` - The break characters, or `null` / `undefined` for the
    ///   default. The argument is required in the TypeScript declaration, as
    ///   for `setSkipWhitespace`.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, enabling method chaining,
    /// or an error for a value that is neither a string nor `null` /
    /// `undefined`.
    #[wasm_bindgen(js_name = "setBreakCharacters")]
    pub fn set_break_characters(
        &self,
        #[wasm_bindgen(unchecked_param_type = "string | null | undefined")] chars: JsValue,
    ) -> Result<TokenizerBuilder, JsValue> {
        let chars = if chars.is_null() || chars.is_undefined() {
            None
        } else {
            Some(chars.as_string().ok_or_else(|| {
                JsValue::from_str("break_characters must be a string, null or undefined")
            })?)
        };
        self.state
            .borrow_mut()
            .inner
            .set_break_characters(chars.as_deref());

        Ok(self.share())
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
        self.state
            .borrow_mut()
            .inner
            .set_space_penalty(&value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

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
        // A path cannot be read in WebAssembly, and the error of the read
        // would not say what to do. A model embedded in the build is not a
        // path: it goes on to the filter.
        #[cfg(feature = "dependency")]
        if name == "japanese_dependency"
            && !a
                .get("model")
                .and_then(Value::as_str)
                .is_some_and(|model| model.starts_with("embedded://"))
        {
            return Err(JsValue::from_str(
                "japanese_dependency cannot read a model from a path in WebAssembly: \
                 give the bytes of the model to appendDependencyFilter, or \
                 embedded://ipadic in a build with embed-dependency-ipadic",
            ));
        }
        self.state.borrow_mut().inner.append_token_filter(name, &a);

        Ok(self.share())
    }

    // Python-style aliases (snake_case)

    /// Sets the tokenization mode (snake_case alias).
    #[wasm_bindgen(js_name = "set_mode")]
    pub fn py_set_mode(&self, mode: &JsMode) -> TokenizerBuilder {
        self.set_mode(mode)
    }

    /// Sets the dictionary by URI (snake_case alias).
    #[wasm_bindgen(js_name = "set_dictionary")]
    pub fn py_set_dictionary(&self, uri: &str) -> TokenizerBuilder {
        self.set_dictionary(uri)
    }

    /// Sets a pre-loaded dictionary instance (snake_case alias).
    #[wasm_bindgen(js_name = "set_dictionary_instance")]
    pub fn py_set_dictionary_instance(&self, dictionary: &JsDictionary) -> TokenizerBuilder {
        self.set_dictionary_instance(dictionary)
    }

    /// Sets a pre-loaded user dictionary instance (snake_case alias).
    #[wasm_bindgen(js_name = "set_user_dictionary_instance")]
    pub fn py_set_user_dictionary_instance(
        &self,
        user_dictionary: &JsUserDictionary,
    ) -> TokenizerBuilder {
        self.set_user_dictionary_instance(user_dictionary)
    }

    /// Sets whether to keep whitespace tokens (snake_case alias).
    #[wasm_bindgen(js_name = "set_keep_whitespace")]
    pub fn py_set_keep_whitespace(&self, keep: bool) -> TokenizerBuilder {
        self.set_keep_whitespace(keep)
    }

    /// Sets whether whitespace is skipped in the lattice (snake_case alias of
    /// `setSkipWhitespace`).
    ///
    /// # Arguments
    ///
    /// * `skip` - A boolean, or `null` / `undefined` for the dictionary's
    ///   default.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration.
    #[wasm_bindgen(js_name = "set_skip_whitespace")]
    pub fn py_set_skip_whitespace(
        &self,
        #[wasm_bindgen(unchecked_param_type = "boolean | null | undefined")] skip: Option<bool>,
    ) -> TokenizerBuilder {
        self.set_skip_whitespace(skip)
    }

    /// Sets the break characters (snake_case alias of `setBreakCharacters`).
    ///
    /// # Arguments
    ///
    /// * `chars` - A string, or `null` / `undefined` for the default.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, or an error for a value
    /// of another type.
    #[wasm_bindgen(js_name = "set_break_characters")]
    pub fn py_set_break_characters(
        &self,
        #[wasm_bindgen(unchecked_param_type = "string | null | undefined")] chars: JsValue,
    ) -> Result<TokenizerBuilder, JsValue> {
        self.set_break_characters(chars)
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

/// The token filter `japanese_dependency`, in a build with the
/// `dependency` feature.
#[cfg(feature = "dependency")]
#[wasm_bindgen]
impl TokenizerBuilder {
    /// Appends the token filter `japanese_dependency`, which writes on each
    /// token its bunsetsu and the bunsetsu that one depends on (the
    /// `dependency` property of a token).
    ///
    /// The other bindings give the filter the path of a model file, with
    /// `appendTokenFilter`; here there is no file system, so the model is
    /// given as its bytes, such as those of a `fetch`.
    ///
    /// # Arguments
    ///
    /// * `model` - The bytes of a model file of `lindera-dependency`, for
    ///   the dictionary of the tokenizer.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, or an error string for
    /// bytes that are not a model with a chunker and a parser.
    #[wasm_bindgen(js_name = "appendDependencyFilter")]
    pub fn append_dependency_filter(&self, model: &[u8]) -> Result<TokenizerBuilder, JsValue> {
        self.state
            .borrow_mut()
            .inner
            .append_dependency_filter(model.to_vec())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(self.share())
    }

    /// Appends the token filter `japanese_dependency` (snake_case alias of
    /// `appendDependencyFilter`).
    ///
    /// # Arguments
    ///
    /// * `model` - The bytes of a model file of `lindera-dependency`.
    ///
    /// # Returns
    ///
    /// A builder handle sharing this configuration, or an error string for
    /// bytes that are not a model with a chunker and a parser.
    #[wasm_bindgen(js_name = "append_dependency_filter")]
    pub fn py_append_dependency_filter(&self, model: &[u8]) -> Result<TokenizerBuilder, JsValue> {
        self.append_dependency_filter(model)
    }
}

/// A tokenizer for morphological analysis.
#[wasm_bindgen]
pub struct Tokenizer {
    /// The backing lindera-binding tokenizer.
    inner: CoreTokenizer,
}

#[wasm_bindgen]
impl Tokenizer {
    /// Creates a tokenizer directly from a loaded dictionary.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - The system dictionary. It is borrowed, so it stays
    ///   usable and can be passed to other tokenizers and builders (#1238).
    /// * `mode` - The `Mode`, as `TokenizerBuilder.setMode` takes it. `null`
    ///   or `undefined` (the default) is normal mode. The `Mode` stays
    ///   usable, so the same one can be passed again.
    /// * `user_dictionary` - The `UserDictionary`, or `null` or `undefined`
    ///   (the default) for none. It stays usable, as `dictionary` does; the
    ///   tokenizer takes a copy of it.
    /// * `space_penalty` - The left-space penalty, in the forms
    ///   `TokenizerBuilder.setSpacePenalty` accepts. `null` or `undefined`
    ///   (the default) keeps the rules the dictionary ships.
    ///
    /// # Returns
    ///
    /// The tokenizer, the error the JS glue throws for a `mode` that is not
    /// a `Mode`, `Error("expected instance of UserDictionary")` for a
    /// `user_dictionary` that is not a `UserDictionary`, or an error string
    /// for an invalid space-penalty setting, as `setSpacePenalty` and
    /// `build()` report it, or for a user dictionary with a context ID
    /// outside the dictionary's connection matrix.
    // Once one parameter carries a type attribute, wasm-bindgen marks a
    // parameter optional in the TypeScript declarations only when it uses
    // `unchecked_optional_param_type`, so every trailing optional parameter
    // needs one; otherwise `new Tokenizer(dictionary)` stops compiling (#1076).
    #[wasm_bindgen(constructor)]
    pub fn new(
        dictionary: &JsDictionary,
        #[wasm_bindgen(unchecked_optional_param_type = "Mode | null")] mode: Option<JsValue>,
        #[wasm_bindgen(unchecked_optional_param_type = "UserDictionary | null")]
        user_dictionary: Option<JsValue>,
        #[wasm_bindgen(
            unchecked_optional_param_type = "boolean | { rules: Array<{ pos: Array<string>; cost: number }> } | null"
        )]
        space_penalty: Option<JsValue>,
    ) -> Result<Tokenizer, JsValue> {
        let mode = mode_from_js(mode)?;
        let user_dictionary = user_dictionary_from_js(user_dictionary)?;
        let space_penalty = match space_penalty {
            Some(value) => js_to_json(value)
                .map_err(|e| JsValue::from_str(&format!("invalid space_penalty: {e}")))?,
            None => Value::Null,
        };
        let inner = CoreTokenizer::from_segmenter_with_space_penalty(
            mode,
            dictionary.inner.clone(),
            user_dictionary,
            &space_penalty,
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
    ///
    /// `n` is an integer of 0 or more, and `cost_threshold` a BigInt that
    /// fits in 64 bits; any other value throws. `cost_threshold` is taken as
    /// a raw `JsValue` because wasm-bindgen would reduce a BigInt passed to
    /// an `i64` parameter modulo 2^64 (lindera/lindera#1209). Once one
    /// parameter carries a type attribute, every trailing optional parameter
    /// needs `unchecked_optional_param_type` to stay optional in the
    /// TypeScript declarations (#1076).
    #[wasm_bindgen(js_name = "tokenizeNbest")]
    pub fn tokenize_nbest(
        &self,
        input_text: &str,
        n: f64,
        #[wasm_bindgen(unchecked_optional_param_type = "boolean | null")] unique: Option<bool>,
        #[wasm_bindgen(unchecked_optional_param_type = "bigint | null")] cost_threshold: Option<
            JsValue,
        >,
    ) -> Result<JsValue, JsValue> {
        let n = integer_arg("n", n)?;
        let cost_threshold = optional_i64_from_bigint("cost_threshold", cost_threshold)?;
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
        n: f64,
        #[wasm_bindgen(unchecked_optional_param_type = "boolean | null")] unique: Option<bool>,
        #[wasm_bindgen(unchecked_optional_param_type = "bigint | null")] cost_threshold: Option<
            JsValue,
        >,
    ) -> Result<JsValue, JsValue> {
        self.tokenize_nbest(input_text, n, unique, cost_threshold)
    }

    /// Tokenizes the input text and returns only the token surfaces (snake_case alias).
    #[wasm_bindgen(js_name = "tokenize_surfaces")]
    pub fn py_tokenize_surfaces(&self, input_text: &str) -> Result<Vec<String>, JsValue> {
        self.tokenize_surfaces(input_text)
    }

    /// Tokenizes every text of `texts`, each as `tokenize` does, and
    /// returns their tokens in the order of `texts`.
    ///
    /// WASM has no threads: the texts are tokenized one after the other on
    /// the calling thread, whatever `max_threads` is. `max_threads` is
    /// checked (an integer of 1 or more) and otherwise ignored, so that a
    /// call written for the other bindings runs unchanged; the batch saves
    /// crossing into WASM once per text.
    ///
    /// # Arguments
    ///
    /// * `texts` - An array of texts.
    /// * `max_threads` - The most threads, an integer of 1 or more, or
    ///   `undefined` / `null`.
    ///
    /// # Returns
    ///
    /// An array with the token objects of each text, or an error for a
    /// `max_threads` that is not an integer of 1 or more.
    #[wasm_bindgen(js_name = "tokenizeBatch")]
    pub fn tokenize_batch(
        &self,
        texts: Vec<String>,
        #[wasm_bindgen(unchecked_optional_param_type = "number | null")] max_threads: Option<f64>,
    ) -> Result<JsValue, JsValue> {
        let max_threads = batch_threads(max_threads)?;
        let batch = self
            .inner
            .tokenize_batch(&texts, max_threads)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let outer = js_sys::Array::new();
        for views in batch {
            outer.push(&tokens_to_js(views));
        }
        Ok(outer.into())
    }

    /// Tokenizes every text of `texts` as `tokenizeSurfaces` does, as
    /// `tokenizeBatch` does.
    ///
    /// # Arguments
    ///
    /// * `texts` - An array of texts.
    /// * `max_threads` - Checked and ignored, as `tokenizeBatch` does.
    ///
    /// # Returns
    ///
    /// An array with the surface strings of each text, or an error for a
    /// `max_threads` that is not an integer of 1 or more.
    #[wasm_bindgen(js_name = "tokenizeSurfacesBatch")]
    pub fn tokenize_surfaces_batch(
        &self,
        texts: Vec<String>,
        #[wasm_bindgen(unchecked_optional_param_type = "number | null")] max_threads: Option<f64>,
    ) -> Result<JsValue, JsValue> {
        let max_threads = batch_threads(max_threads)?;
        let batch = self
            .inner
            .tokenize_surfaces_batch(&texts, max_threads)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let outer = js_sys::Array::new();
        for views in batch {
            let surfaces = js_sys::Array::new();
            for view in views {
                surfaces.push(&JsValue::from(view.surface));
            }
            outer.push(&surfaces);
        }
        Ok(outer.into())
    }

    /// Tokenizes every text of `texts` as `tokenizeNbest` does, as
    /// `tokenizeBatch` does.
    ///
    /// # Arguments
    ///
    /// * `texts` - An array of texts.
    /// * `n` - Number of N-best results per text, an integer of 0 or more.
    /// * `unique` - Deduplicate results with identical word boundaries.
    /// * `cost_threshold` - A BigInt that fits in 64 bits, as
    ///   `tokenizeNbest` takes it.
    /// * `max_threads` - Checked and ignored, as `tokenizeBatch` does.
    ///
    /// # Returns
    ///
    /// An array with the N-best results of each text, each `{ tokens, cost }`
    /// as `tokenizeNbest` returns them, or an error for an `n`, a
    /// `cost_threshold` or a `max_threads` outside its range.
    #[wasm_bindgen(js_name = "tokenizeNbestBatch")]
    pub fn tokenize_nbest_batch(
        &self,
        texts: Vec<String>,
        n: f64,
        #[wasm_bindgen(unchecked_optional_param_type = "boolean | null")] unique: Option<bool>,
        #[wasm_bindgen(unchecked_optional_param_type = "bigint | null")] cost_threshold: Option<
            JsValue,
        >,
        #[wasm_bindgen(unchecked_optional_param_type = "number | null")] max_threads: Option<f64>,
    ) -> Result<JsValue, JsValue> {
        let n = integer_arg("n", n)?;
        let cost_threshold = optional_i64_from_bigint("cost_threshold", cost_threshold)?;
        let max_threads = batch_threads(max_threads)?;
        let batch = self
            .inner
            .tokenize_nbest_batch(
                &texts,
                n,
                unique.unwrap_or(false),
                cost_threshold,
                max_threads,
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let outer = js_sys::Array::new();
        for results in batch {
            let results = nbest_to_js(results)?;
            outer.push(&results);
        }
        Ok(outer.into())
    }

    /// Tokenizes every text of `texts`, as `tokenizeBatch` does
    /// (snake_case alias).
    #[wasm_bindgen(js_name = "tokenize_batch")]
    pub fn py_tokenize_batch(
        &self,
        texts: Vec<String>,
        #[wasm_bindgen(unchecked_optional_param_type = "number | null")] max_threads: Option<f64>,
    ) -> Result<JsValue, JsValue> {
        self.tokenize_batch(texts, max_threads)
    }

    /// Tokenizes every text of `texts`, as `tokenizeSurfacesBatch` does
    /// (snake_case alias).
    #[wasm_bindgen(js_name = "tokenize_surfaces_batch")]
    pub fn py_tokenize_surfaces_batch(
        &self,
        texts: Vec<String>,
        #[wasm_bindgen(unchecked_optional_param_type = "number | null")] max_threads: Option<f64>,
    ) -> Result<JsValue, JsValue> {
        self.tokenize_surfaces_batch(texts, max_threads)
    }

    /// Tokenizes every text of `texts`, as `tokenizeNbestBatch` does
    /// (snake_case alias).
    #[wasm_bindgen(js_name = "tokenize_nbest_batch")]
    pub fn py_tokenize_nbest_batch(
        &self,
        texts: Vec<String>,
        n: f64,
        #[wasm_bindgen(unchecked_optional_param_type = "boolean | null")] unique: Option<bool>,
        #[wasm_bindgen(unchecked_optional_param_type = "bigint | null")] cost_threshold: Option<
            JsValue,
        >,
        #[wasm_bindgen(unchecked_optional_param_type = "number | null")] max_threads: Option<f64>,
    ) -> Result<JsValue, JsValue> {
        self.tokenize_nbest_batch(texts, n, unique, cost_threshold, max_threads)
    }
}

/// Makes the token objects of one text into an array.
///
/// # Arguments
///
/// * `views` - The tokens of the text.
///
/// # Returns
///
/// The array of plain token objects.
fn tokens_to_js(views: Vec<TokenView>) -> js_sys::Array {
    let tokens = js_sys::Array::new();
    for view in views {
        tokens.push(&token_view_to_js(view));
    }
    tokens
}

/// Makes the N-best results of one text into an array of `{ tokens, cost }`
/// objects, as `tokenizeNbest` returns them.
///
/// # Arguments
///
/// * `results` - The `(tokens, cost)` pairs of the text.
///
/// # Returns
///
/// The array, or the error of setting a property.
fn nbest_to_js(results: Vec<(Vec<TokenView>, i64)>) -> Result<js_sys::Array, JsValue> {
    let outer = js_sys::Array::new();
    for (views, cost) in results {
        let entry = js_sys::Object::new();
        js_sys::Reflect::set(&entry, &"tokens".into(), &tokens_to_js(views))?;
        js_sys::Reflect::set(&entry, &"cost".into(), &JsValue::from(cost as f64))?;
        outer.push(&entry);
    }
    Ok(outer)
}

/// Converts the `max_threads` argument of the batch methods, as the other
/// bindings check theirs.
///
/// # Arguments
///
/// * `max_threads` - The argument.
///
/// # Returns
///
/// The thread count, `None` for the default, or an error when it is not an
/// integer of 1 or more.
fn batch_threads(max_threads: Option<f64>) -> Result<Option<NonZeroUsize>, JsValue> {
    Ok(
        optional_integer_arg::<Positive<usize>>("max_threads", max_threads)?
            .and_then(|Positive(n)| NonZeroUsize::new(n)),
    )
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    use crate::mode::JsMode;

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
        builder.set_mode(&JsMode::normal());
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
        builder.set_mode(&JsMode::normal());
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
        builder.set_mode(&JsMode::normal());
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
            .set_mode(&JsMode::normal())
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
        let handle = builder.set_mode(&JsMode::normal());
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
        builder.set_mode(&JsMode::normal());
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
        builder.set_mode(&JsMode::normal());
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
        builder.set_mode(&JsMode::normal());
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
        builder.set_mode(&JsMode::decompose(None).unwrap());
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let tokens = tokenizer.tokenize("関西国際空港").unwrap();

        assert!(!tokens.is_empty());

        let reconstructed: String = tokens.iter().map(|t| token_str(t, "surface")).collect();
        assert_eq!(reconstructed, "関西国際空港");
    }

    /// The penalty of a `Mode` reaches the segmenter through `setMode`, its
    /// alias and the constructor, and the `Mode` stays usable, so one object
    /// is passed again and again (#1215, #1218). Without length penalties,
    /// `関西国際空港` stays whole; with a katakana threshold of 4, the output
    /// differs from both normal mode's and the default penalty's, so a
    /// decompose mode that loses its penalty, or is read as normal mode,
    /// fails.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_decompose_penalty_reaches_the_segmenter() {
        use crate::dictionary::load_dictionary;
        use crate::mode::JsPenalty;
        use crate::{Tokenizer, TokenizerBuilder};
        use wasm_bindgen::JsValue;

        let surfaces = |tokenizer: &Tokenizer| -> Vec<String> {
            tokenizer
                .tokenize("関西国際空港限定トートバッグ")
                .unwrap()
                .iter()
                .map(|t| token_str(t, "surface"))
                .collect()
        };
        let normal = vec!["関西国際空港", "限定", "トートバッグ"];
        let default = vec!["関西", "国際", "空港", "限定", "トートバッグ"];
        let short_katakana = vec!["関西", "国際", "空港", "限定", "トー", "トバッグ"];
        let penalty = |kanji_penalty, other_threshold, other_penalty| {
            JsValue::from(
                JsPenalty::new(None, kanji_penalty, other_threshold, other_penalty).unwrap(),
            )
        };
        let no_length_penalty = penalty(Some(0.0), None, Some(0.0));

        for (mode, expected) in [
            (JsMode::normal(), &normal),
            (JsMode::decompose(None).unwrap(), &default),
            (JsMode::decompose(Some(JsValue::NULL)).unwrap(), &default),
            (
                JsMode::decompose(Some(JsValue::UNDEFINED)).unwrap(),
                &default,
            ),
            (
                JsMode::decompose(Some(no_length_penalty.clone())).unwrap(),
                &normal,
            ),
            (
                JsMode::decompose(Some(penalty(None, Some(4.0), Some(3000.0)))).unwrap(),
                &short_katakana,
            ),
            (JsMode::parse("decompose").unwrap(), &default),
        ] {
            let builder = TokenizerBuilder::new().unwrap();
            builder.set_dictionary("embedded://ipadic");
            builder.set_mode(&mode);
            assert_eq!(&surfaces(&builder.build().unwrap()), expected, "setMode");

            let builder = TokenizerBuilder::new().unwrap();
            builder.set_dictionary("embedded://ipadic");
            builder.py_set_mode(&mode);
            assert_eq!(&surfaces(&builder.build().unwrap()), expected, "set_mode");

            // The same JS object, passed twice: the constructor must not
            // destroy it.
            let mode = JsValue::from(mode);
            for _ in 0..2 {
                let tokenizer = Tokenizer::new(
                    &load_dictionary("embedded://ipadic").unwrap(),
                    Some(mode.clone()),
                    None,
                    None,
                )
                .unwrap();
                assert_eq!(&surfaces(&tokenizer), expected, "constructor");
            }
        }
        // The Penalty passed to Mode.decompose stays usable too.
        assert!(JsMode::decompose(Some(no_length_penalty)).is_ok());
    }

    /// The constructor gives normal mode for a mode left out, `null` or
    /// `undefined`, and rejects a value that is not a `Mode` (#1218).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_constructor_mode_argument() {
        use crate::Tokenizer;
        use crate::dictionary::load_dictionary;
        use crate::mode::JsPenalty;
        use wasm_bindgen::JsValue;

        let build = |mode: Option<JsValue>| {
            Tokenizer::new(
                &load_dictionary("embedded://ipadic").unwrap(),
                mode,
                None,
                None,
            )
        };
        for mode in [None, Some(JsValue::NULL), Some(JsValue::UNDEFINED)] {
            let tokens = build(mode).unwrap().tokenize("関西国際空港").unwrap();
            assert_eq!(tokens.len(), 1);
        }
        for value in [
            JsValue::from_str("decompose"),
            JsValue::from_f64(0.0),
            JsValue::from(JsPenalty::new(None, None, None, None).unwrap()),
        ] {
            let shown = format!("{value:?}");
            assert!(build(Some(value)).is_err(), "{shown}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_empty_string() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_mode(&JsMode::normal());
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
        builder.set_mode(&JsMode::normal());
        builder.set_dictionary("embedded://ipadic");

        let tokenizer = builder.build().unwrap();

        let results = tokenizer
            .tokenize_nbest("すもももももももものうち", 3.0, None, None)
            .unwrap();

        assert!(!results.is_null());
        assert!(!results.is_undefined());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenize_nbest_rejects_integers_that_do_not_fit() {
        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary("embedded://ipadic");
        let tokenizer = builder.build().unwrap();
        let message = |n, cost_threshold| {
            tokenizer
                .tokenize_nbest("東京", n, None, cost_threshold)
                .unwrap_err()
                .as_string()
                .unwrap()
        };
        let bigint = |text: &str| {
            Some(wasm_bindgen::JsValue::from(
                js_sys::BigInt::new(&wasm_bindgen::JsValue::from_str(text)).unwrap(),
            ))
        };

        assert_eq!(
            message(-1.0, None),
            "n must be an integer between 0 and 4294967295, got -1"
        );
        assert_eq!(
            message(4294967297.0, None),
            "n must be an integer between 0 and 4294967295, got 4294967297"
        );
        assert!(message(f64::NAN, None).ends_with("got NaN"));
        assert!(message(1.5, None).ends_with("got 1.5"));
        assert_eq!(
            message(2.0, bigint("18446744073709551616")),
            "cost_threshold must be an integer between -9223372036854775808 and \
             9223372036854775807, got 18446744073709551616"
        );
        assert!(
            tokenizer
                .tokenize_nbest("東京", 2.0, None, bigint("-9223372036854775808"))
                .is_ok()
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_tokenizer_with_dictionary_constructor() {
        use crate::Tokenizer;
        use crate::dictionary::load_dictionary;

        let dict = load_dictionary("embedded://ipadic").unwrap();
        let tokenizer = Tokenizer::new(&dict, Some(JsMode::normal().into()), None, None).unwrap();

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
        builder.set_mode(&JsMode::normal());
        builder.set_dictionary_instance(&dict);

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
        builder.set_mode(&JsMode::decompose(None).unwrap());
        builder.set_dictionary_instance(&dict);

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
        builder.set_dictionary_instance(&dict);

        let tokenizer = builder.build().unwrap();
        let tokens = tokenizer.tokenize("すもも").unwrap();

        assert!(!tokens.is_empty());
    }

    /// Text that each of [`builder_settings`] changes: NFKC folds the
    /// full-width letters, `lowercase` lowers them, and kept whitespace
    /// shows up as a token.
    #[cfg(target_arch = "wasm32")]
    const SETTINGS_TEXT: &str = "Ｌｉｎｄｅｒａ 東京";

    /// A builder setting: a call that changes one setting of the builder.
    #[cfg(target_arch = "wasm32")]
    type BuilderSetting = fn(&crate::TokenizerBuilder);

    /// The builder settings that `build()` must apply whichever way the
    /// dictionary was set, with a name for the assertion messages.
    #[cfg(target_arch = "wasm32")]
    fn builder_settings() -> [(&'static str, BuilderSetting); 3] {
        [
            ("setKeepWhitespace(true)", |builder| {
                builder.set_keep_whitespace(true);
            }),
            ("appendCharacterFilter(unicode_normalize)", |builder| {
                builder
                    .append_character_filter("unicode_normalize", js_json(r#"{"kind":"nfkc"}"#))
                    .unwrap();
            }),
            ("appendTokenFilter(lowercase)", |builder| {
                builder
                    .append_token_filter("lowercase", wasm_bindgen::JsValue::UNDEFINED)
                    .unwrap();
            }),
        ]
    }

    /// Builds a tokenizer with IPADIC, set by URI or as an instance, and
    /// the given settings, and returns the surfaces of [`SETTINGS_TEXT`].
    #[cfg(target_arch = "wasm32")]
    fn build_with_settings(by_instance: bool, settings: &[BuilderSetting]) -> Vec<String> {
        let builder = crate::TokenizerBuilder::new().unwrap();
        if by_instance {
            builder.set_dictionary_instance(
                &crate::dictionary::load_dictionary("embedded://ipadic").unwrap(),
            );
        } else {
            builder.set_dictionary("embedded://ipadic");
        }
        for setting in settings {
            setting(&builder);
        }
        builder
            .build()
            .unwrap()
            .tokenize_surfaces(SETTINGS_TEXT)
            .unwrap()
    }

    /// #1138: `setKeepWhitespace()` and the appended filters apply to a
    /// dictionary set with `setDictionaryInstance()` as they do to one set
    /// with `setDictionary()`.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_dictionary_instance_applies_the_builder_settings() {
        let baseline = build_with_settings(false, &[]);
        assert_eq!(build_with_settings(true, &[]), baseline);

        // Collect every mismatch, so a failure names all the ignored settings.
        let mut mismatches = Vec::new();
        for (name, setting) in builder_settings() {
            let by_uri = build_with_settings(false, &[setting]);
            assert_ne!(
                by_uri, baseline,
                "fixture assumption: {name} changes the output"
            );
            let by_instance = build_with_settings(true, &[setting]);
            if by_instance != by_uri {
                mismatches.push(format!("{name}: {by_instance:?} != {by_uri:?}"));
            }
        }

        let all = builder_settings().map(|(_, setting)| setting);
        let by_uri = build_with_settings(false, &all);
        assert_eq!(by_uri, ["lindera", " ", "東京"]);
        let by_instance = build_with_settings(true, &all);
        if by_instance != by_uri {
            mismatches.push(format!("all: {by_instance:?} != {by_uri:?}"));
        }
        assert!(mismatches.is_empty(), "{mismatches:#?}");
    }

    /// The dictionary set last decides: a dictionary instance set after a
    /// URI is used without loading the URI, and a URI set after an instance
    /// replaces it.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_the_dictionary_set_last_is_used() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;

        // Building from this URI fails, so the build below that succeeds
        // never loaded it.
        let unloadable = "/nonexistent/lindera/dictionary";
        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary(unloadable);
        assert!(builder.build().is_err());

        builder.set_dictionary_instance(&load_dictionary("embedded://ipadic").unwrap());
        let tokenizer = builder.build().unwrap();
        assert_eq!(
            tokenizer.tokenize_surfaces("関西国際空港").unwrap(),
            ["関西国際空港"]
        );

        builder.set_dictionary(unloadable);
        assert!(builder.build().is_err());
    }

    /// #1138: a user dictionary set with `setUserDictionaryInstance()`
    /// applies to a dictionary set with `setDictionary()` as it does to one
    /// set with `setDictionaryInstance()`.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_user_dictionary_instance_applies_with_either_dictionary_setter() {
        use crate::TokenizerBuilder;
        use crate::dictionary::{load_dictionary, load_user_dictionary_from_bytes};

        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let csv = include_bytes!("../../resources/user_dict/ipadic_simple_userdic.csv");
        let mut mismatches = Vec::new();
        for by_instance in [false, true] {
            let builder = TokenizerBuilder::new().unwrap();
            if by_instance {
                builder.set_dictionary_instance(&dictionary);
            } else {
                builder.set_dictionary("embedded://ipadic");
            }
            let surfaces = |builder: &TokenizerBuilder| {
                builder
                    .build()
                    .unwrap()
                    .tokenize_surfaces("東京スカイツリー")
                    .unwrap()
            };
            assert!(
                surfaces(&builder).len() > 1,
                "fixture assumption: 東京スカイツリー splits under bare IPADIC"
            );

            builder.set_user_dictionary_instance(
                &load_user_dictionary_from_bytes(csv, &dictionary.metadata()).unwrap(),
            );
            let with_user_dictionary = surfaces(&builder);
            if with_user_dictionary != ["東京スカイツリー"] {
                mismatches.push(format!("instance: {by_instance}: {with_user_dictionary:?}"));
            }
        }
        assert!(mismatches.is_empty(), "{mismatches:#?}");
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
                    builder.set_dictionary_instance(&load_dictionary("embedded://ipadic").unwrap());
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

    /// `setSkipWhitespace()` and its snake_case alias turn whitespace
    /// skipping off and on, for a dictionary set by URI or as an instance,
    /// and `null` / `undefined` go back to the dictionary's default. IPADIC
    /// reads `が` in `Google が 新しい` as a particle (助詞) with whitespace
    /// skipped, as MeCab does, and as a conjunction (接続詞) without.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_set_skip_whitespace_changes_tokenization() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;

        fn pos_of_ga(builder: &TokenizerBuilder) -> String {
            let readings = surfaces_and_pos(&builder.build().unwrap(), "Google が 新しい");
            match readings.iter().find(|(surface, _)| surface == "が") {
                Some((_, pos)) => pos.clone(),
                None => panic!("no token with surface が: {readings:?}"),
            }
        }

        for by_instance in [false, true] {
            let builder = TokenizerBuilder::new().unwrap();
            if by_instance {
                builder.set_dictionary_instance(&load_dictionary("embedded://ipadic").unwrap());
            } else {
                builder.set_dictionary("embedded://ipadic");
            }
            assert_eq!(
                pos_of_ga(&builder),
                "助詞",
                "default (instance: {by_instance})"
            );
            builder.set_skip_whitespace(Some(false));
            assert_eq!(
                pos_of_ga(&builder),
                "接続詞",
                "false (instance: {by_instance})"
            );
            builder.set_skip_whitespace(None);
            assert_eq!(
                pos_of_ga(&builder),
                "助詞",
                "null (instance: {by_instance})"
            );
            builder.py_set_skip_whitespace(Some(false));
            assert_eq!(
                pos_of_ga(&builder),
                "接続詞",
                "alias false (instance: {by_instance})"
            );
            builder.py_set_skip_whitespace(Some(true));
            assert_eq!(
                pos_of_ga(&builder),
                "助詞",
                "alias true (instance: {by_instance})"
            );
        }
    }

    /// `setBreakCharacters()` and its snake_case alias set the break
    /// characters (#1353), for a dictionary set by URI or as an instance:
    /// `レノン・ジョン`, not in IPADIC, splits at `・` by default and with
    /// `"・"`, is one unknown word with `""` or another set, and `null` /
    /// `undefined` go back to the default. A value of another type is an
    /// error.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_set_break_characters_changes_tokenization() {
        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;
        use wasm_bindgen::JsValue;

        fn surfaces(builder: &TokenizerBuilder) -> Vec<String> {
            surfaces_and_pos(&builder.build().unwrap(), "レノン・ジョン")
                .into_iter()
                .map(|(surface, _)| surface)
                .collect()
        }
        let split = ["レノン", "・", "ジョン"];
        let whole = ["レノン・ジョン"];

        for by_instance in [false, true] {
            let builder = TokenizerBuilder::new().unwrap();
            if by_instance {
                builder.set_dictionary_instance(&load_dictionary("embedded://ipadic").unwrap());
            } else {
                builder.set_dictionary("embedded://ipadic");
            }
            let case = |what: &str| format!("{what} (instance: {by_instance})");
            assert_eq!(surfaces(&builder), split, "{}", case("default"));
            builder.set_break_characters(JsValue::from_str("")).unwrap();
            assert_eq!(surfaces(&builder), whole, "{}", case("empty"));
            builder.set_break_characters(JsValue::NULL).unwrap();
            assert_eq!(surfaces(&builder), split, "{}", case("null"));
            builder
                .set_break_characters(JsValue::from_str("ー"))
                .unwrap();
            assert_eq!(surfaces(&builder), whole, "{}", case("another set"));
            builder.set_break_characters(JsValue::UNDEFINED).unwrap();
            assert_eq!(surfaces(&builder), split, "{}", case("undefined"));
            builder
                .py_set_break_characters(JsValue::from_str(""))
                .unwrap();
            assert_eq!(surfaces(&builder), whole, "{}", case("alias"));
            for value in [JsValue::from_f64(0.0), JsValue::TRUE] {
                assert!(
                    builder.set_break_characters(value).is_err(),
                    "{}",
                    case("not a string")
                );
            }
        }
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
            &Tokenizer::new(&plain, None, None, None).unwrap(),
            SPACED_TEXT,
        );
        let penalized = surfaces_and_pos(
            &Tokenizer::new(&shipping, None, None, None).unwrap(),
            SPACED_TEXT,
        );
        assert_ne!(penalized, baseline);

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary_instance(&shipping);
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
        builder.set_dictionary_instance(&plain);
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

        use lindera::dictionary::core::schema::Schema;

        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;
        use wasm_bindgen::JsValue;

        // More rules than the penalty table can index, on both paths.
        let rule = r#"{"pos":["助詞"],"cost":1}"#;
        let too_many = js_json(&format!(r#"{{"rules":[{}]}}"#, vec![rule; 256].join(",")));
        for by_instance in [false, true] {
            let builder = TokenizerBuilder::new().unwrap();
            if by_instance {
                builder.set_dictionary_instance(&load_dictionary("embedded://ipadic").unwrap());
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
        builder.set_dictionary_instance(&dict);
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

    /// The builder methods that convert a JS value to JSON.
    #[cfg(target_arch = "wasm32")]
    const CONVERTING_METHODS: [&str; 3] = [
        "setSpacePenalty",
        "appendCharacterFilter",
        "appendTokenFilter",
    ];

    /// Part of the error message for an argument that nests too deeply.
    #[cfg(target_arch = "wasm32")]
    const TOO_DEEP: &str = "nested more than 128 levels deep";

    /// Returns `{ a: { a: ... } }` made of `levels` nested objects.
    #[cfg(target_arch = "wasm32")]
    fn nested_objects(levels: usize) -> wasm_bindgen::JsValue {
        let mut value: wasm_bindgen::JsValue = js_sys::Object::new().into();
        for _ in 1..levels {
            let outer = js_sys::Object::new();
            js_sys::Reflect::set(&outer, &"a".into(), &value).unwrap();
            value = outer.into();
        }
        value
    }

    /// Returns `{ rules: [], self: <the object itself> }`.
    #[cfg(target_arch = "wasm32")]
    fn self_containing_object() -> wasm_bindgen::JsValue {
        let object = js_sys::Object::new();
        js_sys::Reflect::set(&object, &"rules".into(), &js_sys::Array::new()).unwrap();
        js_sys::Reflect::set(&object, &"self".into(), &object).unwrap();
        object.into()
    }

    /// Calls one of [`CONVERTING_METHODS`] on a new builder.
    #[cfg(target_arch = "wasm32")]
    fn call_converting_method(method: &str, value: wasm_bindgen::JsValue) -> Result<(), String> {
        let builder = crate::TokenizerBuilder::new().unwrap();
        let result = match method {
            "setSpacePenalty" => builder.set_space_penalty(value),
            "appendCharacterFilter" => builder.append_character_filter("unicode_normalize", value),
            _ => builder.append_token_filter("lowercase", value),
        };
        result
            .map(|_| ())
            .map_err(|error| error.as_string().unwrap_or_default())
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_argument_nesting_limit() {
        for method in CONVERTING_METHODS {
            // setSpacePenalty rejects the shape, but not for being too deep.
            if let Err(message) = call_converting_method(method, nested_objects(128)) {
                assert!(!message.contains(TOO_DEEP), "{method}: {message}");
            }
            for value in [
                nested_objects(129),
                nested_objects(10_000),
                self_containing_object(),
            ] {
                let message = call_converting_method(method, value).expect_err(method);
                assert!(message.contains(TOO_DEEP), "{method}: {message}");
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_argument_error_leaves_the_module_usable() {
        for method in CONVERTING_METHODS {
            assert!(call_converting_method(method, self_containing_object()).is_err());
            // Without the limit, the stack overflow trapped the module and
            // every later call into it failed as well.
            assert!(
                call_converting_method(method, wasm_bindgen::JsValue::NULL).is_ok(),
                "{method}"
            );
            assert!(!crate::version().is_empty());
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_map_keyed_by_a_self_containing_array_is_rejected() {
        let key = js_sys::Array::new();
        key.push(&key);
        let map = js_sys::Map::new();
        map.set(&key, &1.into());
        let message = call_converting_method("appendTokenFilter", map.into())
            .expect_err("a non-string key is rejected");
        assert!(message.contains("expected a string key"), "{message}");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_argument_conversion_matches_serde_wasm_bindgen() {
        let map = js_sys::Map::new();
        map.set(&"k".into(), &js_json(r#"{"n":[1,2]}"#));
        for value in [
            js_json(r#"{"a":[1,-2,2.5,"x",null,true,{"b":{}}],"c":"日本語"}"#),
            js_sys::eval("({ a: undefined, b: NaN, c: 10n, d: [Infinity, -0] })").unwrap(),
            map.into(),
            js_sys::Uint8Array::new_with_length(2).into(),
            js_sys::Set::new(&wasm_bindgen::JsValue::UNDEFINED).into(),
        ] {
            let shown = format!("{value:?}");
            let expected = serde_wasm_bindgen::from_value::<serde_json::Value>(value.clone())
                .map_err(|error| error.to_string());
            let actual = crate::json::json_from_js(value).map_err(|error| error.to_string());
            assert_eq!(actual, expected, "{shown}");
        }
    }

    /// The constructor takes the same space-penalty setting as
    /// `TokenizerBuilder.setSpacePenalty`, and omitting it keeps the
    /// dictionary's default.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_constructor_applies_a_space_penalty_setting() {
        use std::sync::Arc;

        use crate::Tokenizer;
        use crate::dictionary::{JsDictionary, load_dictionary};
        use wasm_bindgen::JsValue;

        let plain = load_dictionary("embedded://ipadic").unwrap();
        let mut shipping = plain.clone();
        Arc::make_mut(&mut shipping.inner.metadata).space_penalty = Some(ipadic_rules_config());
        let tokenize = |dictionary: &JsDictionary, space_penalty: Option<JsValue>| {
            let tokenizer = Tokenizer::new(dictionary, None, None, space_penalty).unwrap();
            surfaces_and_pos(&tokenizer, SPACED_TEXT)
        };

        let baseline = tokenize(&plain, None);
        let penalized = tokenize(&shipping, None);
        assert_ne!(penalized, baseline);

        for (value, expected) in [
            (JsValue::NULL, &penalized),
            (JsValue::UNDEFINED, &penalized),
            (JsValue::FALSE, &baseline),
            (JsValue::TRUE, &penalized),
            (js_json(r#"{"rules":[]}"#), &baseline),
        ] {
            let shown = format!("{value:?}");
            assert_eq!(&tokenize(&shipping, Some(value)), expected, "value {shown}");
        }
        // Explicit rules apply to a dictionary that ships none.
        assert_eq!(tokenize(&plain, Some(ipadic_rules())), penalized);
    }

    /// The constructor reports an invalid setting as `setSpacePenalty` and
    /// `build()` do.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_constructor_rejects_an_invalid_space_penalty() {
        use crate::Tokenizer;
        use crate::dictionary::load_dictionary;
        use wasm_bindgen::JsValue;

        let error = |space_penalty: JsValue| {
            let dictionary = load_dictionary("embedded://ipadic").unwrap();
            match Tokenizer::new(&dictionary, None, None, Some(space_penalty)) {
                Ok(_) => panic!("the setting was accepted"),
                Err(error) => error.as_string().unwrap_or_default(),
            }
        };

        // IPADIC ships no rules to require.
        assert!(error(JsValue::TRUE).contains("ships no space_penalty rules"));
        for value in [
            JsValue::from(1),
            JsValue::from_str("false"),
            js_json(r#"{"rules":"JKS"}"#),
        ] {
            let shown = format!("{value:?}");
            assert!(error(value).contains("space_penalty"), "value {shown}");
        }
    }

    /// `setUserDictionaryInstance()` shares the user dictionary with the
    /// caller's object instead of copying its automaton and data; `build()`
    /// copies it for the tokenizer (#1238).
    #[test]
    fn test_set_user_dictionary_instance_shares_the_user_dictionary() {
        use std::rc::Rc;

        use crate::TokenizerBuilder;
        use crate::dictionary::load_user_dictionary_bin_from_bytes;

        let bin = include_bytes!("../../resources/user_dict/ipadic_simple_userdic.bin");
        let user_dictionary = load_user_dictionary_bin_from_bytes(bin).unwrap();
        let builder = TokenizerBuilder::new().unwrap();
        builder.set_user_dictionary_instance(&user_dictionary);
        let state = builder.state.borrow();
        let Some(set) = &state.user_dictionary_instance else {
            panic!("no user dictionary was set");
        };
        assert!(Rc::ptr_eq(&set.inner, &user_dictionary.inner));
    }

    /// The user dictionary of the tests that pass one dictionary object to
    /// several calls: it keeps `東京スカイツリー` in one token, which IPADIC
    /// splits.
    #[cfg(target_arch = "wasm32")]
    const SKYTREE_CSV: &str = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー\n";

    /// Builds a tokenizer with IPADIC set by URI, for [`construct_like`] to
    /// reach the `Tokenizer` class from.
    ///
    /// [`construct_like`]: crate::test_utils::construct_like
    #[cfg(target_arch = "wasm32")]
    fn any_tokenizer() -> wasm_bindgen::JsValue {
        let builder = crate::TokenizerBuilder::new().unwrap();
        builder.set_dictionary("embedded://ipadic");
        builder.build().unwrap().into()
    }

    /// Tokenizes `text` with a JS tokenizer through JS.
    #[cfg(target_arch = "wasm32")]
    fn js_surfaces(
        tokenizer: Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>,
        text: &str,
    ) -> Result<Vec<String>, wasm_bindgen::JsValue> {
        use crate::test_utils::{call_method, strings};

        let surfaces = call_method(&tokenizer?, "tokenizeSurfaces", &[&text.into()])?;
        Ok(strings(&surfaces))
    }

    /// The constructor leaves its dictionary usable, so one dictionary serves
    /// several tokenizers (#1238). Called through JS: the JS glue, which a
    /// call from Rust skips, destroyed a dictionary taken by value.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_constructor_leaves_the_dictionary_usable() {
        use wasm_bindgen::JsValue;

        use crate::dictionary::load_dictionary;
        use crate::test_utils::construct_like;

        let any_tokenizer = any_tokenizer();
        let dictionary = JsValue::from(load_dictionary("embedded://ipadic").unwrap());
        let mut failures = Vec::new();
        for attempt in 0..2 {
            let tokenizer = construct_like(&any_tokenizer, &[&dictionary]);
            let surfaces = js_surfaces(tokenizer, "関西国際空港");
            if !matches!(&surfaces, Ok(surfaces) if surfaces == &["関西国際空港"]) {
                failures.push(format!("attempt {attempt}: {surfaces:?}"));
            }
        }
        if let Err(error) = js_sys::Reflect::get(&dictionary, &"name".into()) {
            failures.push(format!("dictionary.name afterwards: {error:?}"));
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// The constructor leaves its user dictionary usable, so one user
    /// dictionary serves several tokenizers, and applies in each (#1238).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_constructor_leaves_the_user_dictionary_usable() {
        use wasm_bindgen::JsValue;

        use crate::dictionary::{load_dictionary, load_user_dictionary_from_bytes_for};
        use crate::test_utils::construct_like;

        let any_tokenizer = any_tokenizer();
        let user_dictionary = JsValue::from(
            load_user_dictionary_from_bytes_for(
                SKYTREE_CSV.as_bytes(),
                &load_dictionary("embedded://ipadic").unwrap(),
            )
            .unwrap(),
        );
        let mut failures = Vec::new();
        for attempt in 0..2 {
            // A new dictionary each time: only the user dictionary is passed
            // again.
            let dictionary = JsValue::from(load_dictionary("embedded://ipadic").unwrap());
            let tokenizer = construct_like(
                &any_tokenizer,
                &[&dictionary, &JsValue::UNDEFINED, &user_dictionary],
            );
            let surfaces = js_surfaces(tokenizer, "東京スカイツリー");
            if !matches!(&surfaces, Ok(surfaces) if surfaces == &["東京スカイツリー"]) {
                failures.push(format!("attempt {attempt}: {surfaces:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// `setDictionaryInstance()` and its alias leave their dictionary usable,
    /// so one dictionary serves several builders (#1238).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_set_dictionary_instance_leaves_the_dictionary_usable() {
        use wasm_bindgen::JsValue;

        use crate::TokenizerBuilder;
        use crate::dictionary::load_dictionary;
        use crate::test_utils::call_method;

        let mut failures = Vec::new();
        for method in ["setDictionaryInstance", "set_dictionary_instance"] {
            let dictionary = JsValue::from(load_dictionary("embedded://ipadic").unwrap());
            for attempt in 0..2 {
                let builder = JsValue::from(TokenizerBuilder::new().unwrap());
                let tokenizer = call_method(&builder, method, &[&dictionary])
                    .and_then(|_| call_method(&builder, "build", &[]));
                let surfaces = js_surfaces(tokenizer, "関西国際空港");
                if !matches!(&surfaces, Ok(surfaces) if surfaces == &["関西国際空港"]) {
                    failures.push(format!("{method}, attempt {attempt}: {surfaces:?}"));
                }
            }
            if let Err(error) = js_sys::Reflect::get(&dictionary, &"name".into()) {
                failures.push(format!("dictionary.name after {method}: {error:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// `setUserDictionaryInstance()` and its alias leave their user
    /// dictionary usable, so one user dictionary serves several builders,
    /// and applies in each (#1238).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_set_user_dictionary_instance_leaves_the_user_dictionary_usable() {
        use wasm_bindgen::JsValue;

        use crate::TokenizerBuilder;
        use crate::dictionary::{load_dictionary, load_user_dictionary_from_bytes_for};
        use crate::test_utils::call_method;

        let mut failures = Vec::new();
        for method in ["setUserDictionaryInstance", "set_user_dictionary_instance"] {
            let user_dictionary = JsValue::from(
                load_user_dictionary_from_bytes_for(
                    SKYTREE_CSV.as_bytes(),
                    &load_dictionary("embedded://ipadic").unwrap(),
                )
                .unwrap(),
            );
            for attempt in 0..2 {
                // The dictionary is set by URI: only the user dictionary is
                // passed again.
                let builder = TokenizerBuilder::new().unwrap();
                builder.set_dictionary("embedded://ipadic");
                let builder = JsValue::from(builder);
                let tokenizer = call_method(&builder, method, &[&user_dictionary])
                    .and_then(|_| call_method(&builder, "build", &[]));
                let surfaces = js_surfaces(tokenizer, "東京スカイツリー");
                if !matches!(&surfaces, Ok(surfaces) if surfaces == &["東京スカイツリー"]) {
                    failures.push(format!("{method}, attempt {attempt}: {surfaces:?}"));
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// `JSON.stringify` of a value: tokens and N-best results are plain
    /// objects, so equal results give equal strings.
    #[cfg(target_arch = "wasm32")]
    fn json(value: &wasm_bindgen::JsValue) -> String {
        js_sys::JSON::stringify(value).unwrap().as_string().unwrap()
    }

    /// The batch methods give every text the results of the single calls,
    /// in the order of the texts, through their camelCase names and their
    /// snake_case aliases, with and without `max_threads` (#1402).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_batch_methods_match_single_calls() {
        use wasm_bindgen::JsValue;

        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary("embedded://ipadic");
        let tokenizer = builder.build().unwrap();
        let texts: Vec<String> = [
            "関西国際空港限定トートバッグ",
            "すもももももももものうち",
            "",
        ]
        .iter()
        .map(|text| text.to_string())
        .collect();

        let expected = json(&JsValue::from(
            texts
                .iter()
                .map(|text| {
                    tokenizer
                        .tokenize(text)
                        .unwrap()
                        .into_iter()
                        .collect::<js_sys::Array>()
                })
                .collect::<js_sys::Array>(),
        ));
        for batch in [
            tokenizer.tokenize_batch(texts.clone(), None).unwrap(),
            tokenizer.tokenize_batch(texts.clone(), Some(4.0)).unwrap(),
            tokenizer.py_tokenize_batch(texts.clone(), None).unwrap(),
        ] {
            assert_eq!(json(&batch), expected);
        }

        let expected_surfaces = json(&JsValue::from(
            texts
                .iter()
                .map(|text| {
                    tokenizer
                        .tokenize_surfaces(text)
                        .unwrap()
                        .into_iter()
                        .map(JsValue::from)
                        .collect::<js_sys::Array>()
                })
                .collect::<js_sys::Array>(),
        ));
        for batch in [
            tokenizer
                .tokenize_surfaces_batch(texts.clone(), Some(2.0))
                .unwrap(),
            tokenizer
                .py_tokenize_surfaces_batch(texts.clone(), None)
                .unwrap(),
        ] {
            assert_eq!(json(&batch), expected_surfaces);
        }

        let expected_nbest = json(&JsValue::from(
            texts
                .iter()
                .map(|text| {
                    tokenizer
                        .tokenize_nbest(text, 3.0, Some(true), None)
                        .unwrap()
                })
                .collect::<js_sys::Array>(),
        ));
        for batch in [
            tokenizer
                .tokenize_nbest_batch(texts.clone(), 3.0, Some(true), None, Some(2.0))
                .unwrap(),
            tokenizer
                .py_tokenize_nbest_batch(texts.clone(), 3.0, Some(true), None, None)
                .unwrap(),
        ] {
            assert_eq!(json(&batch), expected_nbest);
        }
    }

    /// The batch methods reject a `max_threads` that is not an integer of 1
    /// or more, as the other bindings do (#1402).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_batch_methods_reject_a_thread_count_below_one() {
        use wasm_bindgen::JsValue;

        use crate::TokenizerBuilder;

        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary("embedded://ipadic");
        let tokenizer = builder.build().unwrap();
        let texts = || vec!["東京".to_string()];
        let message = |result: Result<JsValue, JsValue>| result.unwrap_err().as_string().unwrap();

        let zero = "max_threads must be an integer between 1 and 4294967295, got 0";
        assert_eq!(message(tokenizer.tokenize_batch(texts(), Some(0.0))), zero);
        assert_eq!(
            message(tokenizer.tokenize_surfaces_batch(texts(), Some(0.0))),
            zero
        );
        assert_eq!(
            message(tokenizer.tokenize_nbest_batch(texts(), 2.0, None, None, Some(0.0))),
            zero
        );
        assert_eq!(
            message(tokenizer.tokenize_batch(texts(), Some(-1.0))),
            "max_threads must be an integer between 1 and 4294967295, got -1"
        );
        assert!(message(tokenizer.tokenize_batch(texts(), Some(1.5))).ends_with("got 1.5"));
    }
}

/// The token filter `japanese_dependency` (#1438), with the model file of
/// the tests of lindera-analysis: every bunsetsu depends on the next one.
#[cfg(all(test, target_arch = "wasm32", feature = "dependency"))]
mod dependency_tests {
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::wasm_bindgen_test;

    use crate::TokenizerBuilder;

    const MODEL: &[u8] = include_bytes!("../../resources/dependency/test_model_ipadic.bin");
    const TEXT: &str = "彼は本を読んだ。";

    /// A builder with the embedded IPADIC.
    fn builder() -> TokenizerBuilder {
        let builder = TokenizerBuilder::new().unwrap();
        builder.set_dictionary("embedded://ipadic");
        builder
    }

    /// The property `key` of an object.
    fn get(object: &JsValue, key: &str) -> JsValue {
        js_sys::Reflect::get(object, &key.into()).unwrap()
    }

    /// The names of the properties of an object, in their order.
    fn keys(object: &JsValue) -> Vec<String> {
        js_sys::Object::keys(&js_sys::Object::from(object.clone()))
            .iter()
            .map(|key| key.as_string().unwrap())
            .collect()
    }

    /// The surface of a token, and the bunsetsu, the head, the role and the
    /// score of its dependency.
    fn written(token: &JsValue) -> (String, f64, Option<f64>, String, f64) {
        let dependency = get(token, "dependency");
        (
            get(token, "surface").as_string().unwrap(),
            get(&dependency, "bunsetsu").as_f64().unwrap(),
            get(&dependency, "head").as_f64(),
            get(&dependency, "role").as_string().unwrap(),
            get(&dependency, "score").as_f64().unwrap(),
        )
    }

    #[wasm_bindgen_test]
    fn a_token_has_its_dependency() {
        let tokenizer = builder()
            .append_dependency_filter(MODEL)
            .unwrap()
            .build()
            .unwrap();
        let tokens = tokenizer.tokenize(TEXT).unwrap();
        let expected = [
            ("彼", 0.0, Some(1.0), "head", 1.0),
            ("は", 0.0, Some(1.0), "function", 1.0),
            ("本", 1.0, Some(2.0), "head", 0.0),
            ("を", 1.0, Some(2.0), "function", 0.0),
            ("読ん", 2.0, None, "head", 0.0),
            ("だ", 2.0, None, "function", 0.0),
            ("。", 2.0, None, "other", 0.0),
        ];
        assert_eq!(tokens.len(), expected.len());
        for (token, (surface, bunsetsu, head, role, score)) in tokens.iter().zip(expected) {
            assert_eq!(
                written(token),
                (surface.to_string(), bunsetsu, head, role.to_string(), score)
            );
        }
        // The fields in their order; the head of the last bunsetsu is null,
        // not missing.
        assert_eq!(
            keys(&tokens[6]),
            [
                "surface",
                "byteStart",
                "byteEnd",
                "position",
                "wordId",
                "isUnknown",
                "details",
                "dependency"
            ]
        );
        let last = get(&tokens[6], "dependency");
        assert_eq!(
            keys(&last),
            ["sentence", "bunsetsu", "head", "role", "score"]
        );
        assert!(get(&last, "head").is_null());
        assert_eq!(get(&last, "sentence").as_f64(), Some(0.0));

        // A second sentence is numbered 1, and its bunsetsu from 0.
        let tokens = tokenizer.tokenize("彼は本を読んだ。本を読んだ。").unwrap();
        let second = get(&tokens[7], "dependency");
        assert_eq!(get(&second, "sentence").as_f64(), Some(1.0));
        assert_eq!(get(&second, "bunsetsu").as_f64(), Some(0.0));
    }

    #[wasm_bindgen_test]
    fn a_token_without_the_filter_has_no_such_property() {
        let tokens = builder().build().unwrap().tokenize(TEXT).unwrap();
        for token in &tokens {
            assert!(get(token, "dependency").is_undefined());
            assert_eq!(keys(token).len(), 7);
        }
    }

    #[wasm_bindgen_test]
    fn the_filter_runs_where_it_was_appended() {
        let stop =
            serde_wasm_bindgen::to_value(&serde_json::json!({"tags": ["助詞,係助詞"]})).unwrap();
        // Before a filter that removes は.
        let tokens = builder()
            .append_dependency_filter(MODEL)
            .unwrap()
            .append_token_filter("japanese_stop_tags", stop.clone())
            .unwrap()
            .build()
            .unwrap()
            .tokenize(TEXT)
            .unwrap();
        assert_eq!(tokens.len(), 6);
        assert_eq!(written(&tokens[1]).0, "本");
        // After it, the tokens are refused.
        let error = builder()
            .append_token_filter("japanese_stop_tags", stop)
            .unwrap()
            .append_dependency_filter(MODEL)
            .unwrap()
            .build()
            .unwrap()
            .tokenize(TEXT)
            .unwrap_err();
        assert!(error.as_string().unwrap().contains("removed or joined"));
    }

    #[wasm_bindgen_test]
    fn what_is_not_a_model_and_a_path_are_refused() {
        // Bytes that are not a model, when they are given.
        assert!(builder().append_dependency_filter(b"not a model").is_err());
        // A path cannot be read here: the error names the method to use.
        let args =
            serde_wasm_bindgen::to_value(&serde_json::json!({"model": "ipadic.model"})).unwrap();
        let error = builder()
            .append_token_filter("japanese_dependency", args)
            .err()
            .unwrap();
        assert!(
            error
                .as_string()
                .unwrap()
                .contains("appendDependencyFilter")
        );
    }

    #[wasm_bindgen_test]
    fn an_embedded_model_is_not_taken_for_a_path() {
        // No model, and a model that is not a string, are refused as a path.
        for args in [serde_json::json!({}), serde_json::json!({"model": 1})] {
            let error = builder()
                .append_token_filter(
                    "japanese_dependency",
                    serde_wasm_bindgen::to_value(&args).unwrap(),
                )
                .err()
                .unwrap();
            assert!(
                error
                    .as_string()
                    .unwrap()
                    .contains("appendDependencyFilter")
            );
        }
        // `embedded://` goes on to the filter, which answers for the build.
        let args = serde_wasm_bindgen::to_value(&serde_json::json!({"model": "embedded://ipadic"}))
            .unwrap();
        let built = builder()
            .append_token_filter("japanese_dependency", args)
            .unwrap()
            .build();
        #[cfg(not(feature = "embed-dependency-ipadic"))]
        assert!(format!("{:?}", built.err().unwrap()).contains("embed-dependency-ipadic"));
        #[cfg(feature = "embed-dependency-ipadic")]
        assert_eq!(
            written(&built.unwrap().tokenize(TEXT).unwrap()[0]).3,
            "head"
        );
    }

    #[wasm_bindgen_test]
    fn the_snake_case_alias_appends_the_filter() {
        let tokens = builder()
            .py_append_dependency_filter(MODEL)
            .unwrap()
            .build()
            .unwrap()
            .tokenize(TEXT)
            .unwrap();
        assert_eq!(written(&tokens[0]).3, "head");
    }
}
