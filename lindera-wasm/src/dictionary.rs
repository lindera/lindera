use std::rc::Rc;
use std::sync::Arc;

use wasm_bindgen::JsCast;
use wasm_bindgen::convert::TryFromJsValue;
use wasm_bindgen::prelude::*;

use lindera::dictionary::core::character_definition::CharacterDefinition;
use lindera::dictionary::core::connection_cost_matrix::ConnectionCostMatrix;
use lindera::dictionary::core::metadata::Metadata;
use lindera::dictionary::core::prefix_dictionary::PrefixDictionary;
use lindera::dictionary::core::unknown_dictionary::UnknownDictionary;
use lindera::dictionary::{
    Dictionary, DictionaryBuilder, UserDictionary, load_dictionary as lindera_load_dictionary,
};

use crate::metadata::JsMetadata;

/// A morphological analysis dictionary.
#[wasm_bindgen(js_name = "Dictionary")]
#[derive(Clone)]
pub struct JsDictionary {
    pub(crate) inner: Dictionary,
}

#[wasm_bindgen(js_class = "Dictionary")]
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
        JsMetadata::from((*self.inner.metadata).clone())
    }
}

/// A user-defined dictionary for custom words.
///
/// It stays usable when it is passed to the `Tokenizer` constructor or to
/// `TokenizerBuilder.setUserDictionaryInstance()`, so one user dictionary can
/// serve several tokenizers and builders (#1238).
#[wasm_bindgen(js_name = "UserDictionary")]
#[derive(Clone)]
pub struct JsUserDictionary {
    /// The user dictionary, shared with the builders it is set on and with
    /// the copies `__copyOf` returns. A clone of a `UserDictionary` copies
    /// its automaton and data, unlike one of a `Dictionary`.
    pub(crate) inner: Rc<UserDictionary>,
}

impl JsUserDictionary {
    /// Wraps a user dictionary.
    pub fn new(inner: UserDictionary) -> Self {
        Self {
            inner: Rc::new(inner),
        }
    }

    /// Returns a copy of the user dictionary, for a tokenizer, whose
    /// segmenter owns its user dictionary.
    pub(crate) fn to_lindera(&self) -> UserDictionary {
        UserDictionary::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = "UserDictionary")]
impl JsUserDictionary {
    /// Returns a handle sharing the payload of `user_dictionary`, for
    /// [`user_dictionary_from_js`]; hidden from the TypeScript declarations.
    ///
    /// Called through the JS glue, which checks that `user_dictionary` is a
    /// `UserDictionary` and, as it is a reference, leaves it usable.
    #[wasm_bindgen(js_name = "__copyOf", skip_typescript)]
    pub fn copy_of(user_dictionary: &JsUserDictionary) -> JsUserDictionary {
        user_dictionary.clone()
    }
}

/// Reads the optional `UserDictionary` argument of the `Tokenizer`
/// constructor (#1238).
///
/// wasm-bindgen takes no `Option<&UserDictionary>`, and an
/// `Option<UserDictionary>` destroys the JS object of the argument, so the
/// caller could not pass the same `UserDictionary` again. The argument is
/// taken as a `JsValue` and passed to `UserDictionary.__copyOf` through the
/// JS glue instead, as [`crate::mode::mode_from_js`] does for a `Mode`, and
/// only the copy is taken by value. `__copyOf` is found through the
/// argument's `constructor`, as no `UserDictionary` can be created cheaply to
/// reach the class from. The copy is checked to be a `UserDictionary`, so a
/// value of another class with a `__copyOf`, such as a `Mode`, is rejected.
///
/// # Arguments
///
/// * `value` - The argument, or `None` when it is left out.
///
/// # Returns
///
/// `None` for an argument that is left out, `null` or `undefined`, a copy of
/// the user dictionary otherwise, or `Error("expected instance of
/// UserDictionary")`, which the glue throws for any other class argument, for
/// a value that is not a `UserDictionary`.
pub fn user_dictionary_from_js(value: Option<JsValue>) -> Result<Option<UserDictionary>, JsValue> {
    let Some(value) = value.filter(|value| !value.is_null() && !value.is_undefined()) else {
        return Ok(None);
    };
    let expected = || JsValue::from(js_sys::Error::new("expected instance of UserDictionary"));
    // `Reflect.get` throws for a value that is not an object, and for one
    // whose `constructor` is not.
    let copy_of = js_sys::Reflect::get(&value, &"constructor".into())
        .and_then(|class| js_sys::Reflect::get(&class, &"__copyOf".into()))
        .ok()
        .and_then(|copy_of| copy_of.dyn_into::<js_sys::Function>().ok())
        .ok_or_else(expected)?;
    let copy = copy_of.call1(&JsValue::UNDEFINED, &value)?;
    let user_dictionary = JsUserDictionary::try_from_js_value(copy).map_err(|_| expected())?;
    Ok(Some(user_dictionary.to_lindera()))
}

/// Loads a dictionary from the specified URI.
///
/// On WebAssembly only `embedded://` dictionaries can be loaded by URI:
/// there is no filesystem on `wasm32-unknown-unknown`, so file URIs and
/// bare paths are rejected up front with a pointer at the bytes API (#971).
#[wasm_bindgen(js_name = "loadDictionary")]
pub fn load_dictionary(uri: &str) -> Result<JsDictionary, JsValue> {
    if !uri.starts_with("embedded://") {
        return Err(JsValue::from_str(
            "filesystem dictionaries are not available in WebAssembly; \
             use loadDictionaryFromBytes() with bytes obtained in JavaScript \
             (see the OPFS helper in js/opfs.js), or an embedded:// dictionary",
        ));
    }
    let dict = lindera_load_dictionary(uri).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsDictionary { inner: dict })
}

/// Loads a dictionary from raw byte arrays.
///
/// This function constructs a `Dictionary` directly from the binary data
/// of each dictionary component file, without requiring filesystem access.
/// This is useful for loading dictionaries from OPFS or other browser storage.
///
/// # Arguments
///
/// * `metadata` - The contents of `metadata.json`
/// * `dict_trie` - The contents of `dict.trie` (char-wise double-array trie)
/// * `dict_vals_idx` - The contents of `dict.valsidx` (values index)
/// * `dict_vals` - The contents of `dict.vals` (word value data)
/// * `dict_words_idx` - The contents of `dict.wordsidx` (word details index)
/// * `dict_words` - The contents of `dict.words` (word details)
/// * `matrix_mtx` - The contents of `matrix.mtx` (connection cost matrix)
/// * `char_def` - The contents of `char_def.bin` (character definitions)
/// * `unk` - The contents of `unk.bin` (unknown word dictionary)
///
/// # Returns
///
/// A `Dictionary` instance constructed from the provided byte data.
///
/// # Breaking changes (dictionary format versions 2 and 3)
///
/// Dictionaries built before Lindera v6 shipped a byte-wise automaton as
/// `dict.da`; that argument was replaced by `dict_trie` + `dict_vals_idx`.
/// Dictionaries built before Lindera v7 (format version 2) list the
/// categories in `char_def.bin` in another order (#1111) and are rejected.
/// Rebuild the dictionary or download a matching prebuilt one, including
/// one saved in OPFS by an older Lindera.
#[wasm_bindgen(js_name = "loadDictionaryFromBytes")]
#[allow(clippy::too_many_arguments)]
pub fn load_dictionary_from_bytes(
    metadata: &[u8],
    dict_trie: &[u8],
    dict_vals_idx: &[u8],
    dict_vals: &[u8],
    dict_words_idx: &[u8],
    dict_words: &[u8],
    matrix_mtx: &[u8],
    char_def: &[u8],
    unk: &[u8],
) -> Result<JsDictionary, JsValue> {
    let meta =
        Metadata::load(metadata).map_err(|e| JsValue::from_str(&format!("metadata: {e}")))?;
    // These bytes come from OPFS or a fetch, so they can easily be a
    // dictionary built by an older Lindera. Every other argument below is a
    // headerless raw array that would decode into garbage rather than fail.
    meta.validate_format_version()
        .map_err(|e| JsValue::from_str(&format!("metadata: {e}")))?;

    let prefix_dictionary = PrefixDictionary::load(
        dict_trie.to_vec(),
        dict_vals_idx.to_vec(),
        dict_vals.to_vec(),
        dict_words_idx.to_vec(),
        dict_words.to_vec(),
    )
    .map_err(|e| JsValue::from_str(&format!("prefix_dict: {e}")))?;
    let connection_cost_matrix = ConnectionCostMatrix::load(matrix_mtx.to_vec())
        .map_err(|e| JsValue::from_str(&format!("connection: {e}")))?;
    let character_definition = CharacterDefinition::load(char_def)
        .map_err(|e| JsValue::from_str(&format!("char_def: {e}")))?;
    let unknown_dictionary =
        UnknownDictionary::load(unk).map_err(|e| JsValue::from_str(&format!("unk: {e}")))?;

    let dict = Dictionary {
        prefix_dictionary: Arc::new(prefix_dictionary),
        connection_cost_matrix: Arc::new(connection_cost_matrix),
        character_definition: Arc::new(character_definition),
        unknown_dictionary: Arc::new(unknown_dictionary),
        metadata: Arc::new(meta),
    };

    Ok(JsDictionary { inner: dict })
}

/// Builds a user dictionary from CSV bytes.
///
/// The bytes typically come from `fetch`, an `<input type="file">` element,
/// or OPFS. The content must be UTF-8. Rows follow either the simple format
/// defined by the metadata's user dictionary schema (for IPADIC:
/// `surface,part_of_speech,reading`) or the full dictionary format; pass the
/// metadata of the system dictionary the user dictionary will be used with
/// (e.g. `dictionary.metadata`).
///
/// The result feeds `Tokenizer`'s user-dictionary argument or
/// `TokenizerBuilder.setUserDictionaryInstance()`.
///
/// # Arguments
///
/// * `csv` - The user dictionary CSV content (UTF-8).
/// * `metadata` - The metadata the CSV is built with. It is borrowed, so it
///   stays usable and can be passed again (#1238).
///
/// # Returns
///
/// The user dictionary, or an error if the CSV cannot be built.
#[wasm_bindgen(js_name = "loadUserDictionaryFromBytes")]
pub fn load_user_dictionary_from_bytes(
    csv: &[u8],
    metadata: &JsMetadata,
) -> Result<JsUserDictionary, JsValue> {
    let meta: Metadata = metadata.clone().into();
    let builder = DictionaryBuilder::new(meta);
    let dict = builder
        .build_user_dict_from_reader(csv)
        .map_err(|e| JsValue::from_str(&format!("user_dict csv: {e:?}")))?;
    Ok(JsUserDictionary::new(dict))
}

/// Builds a user dictionary from CSV bytes for a loaded system dictionary.
///
/// The CSV is built with the system dictionary's metadata, as
/// `loadUserDictionaryFromBytes` builds it with the metadata passed, and is
/// checked against the system dictionary's connection matrix as it is
/// built: a row whose context ID lies outside the matrix fails with an error
/// that names its CSV line and surface. The system dictionary is borrowed,
/// so it can still be passed to `Tokenizer` or
/// `TokenizerBuilder.setDictionaryInstance()` afterwards.
///
/// # Arguments
///
/// * `csv` - The user dictionary CSV content (UTF-8).
/// * `dictionary` - The system dictionary the user dictionary is used with.
///
/// # Returns
///
/// The user dictionary, or an error if the CSV cannot be built or a row has
/// a context ID outside the system dictionary's connection matrix.
#[wasm_bindgen(js_name = "loadUserDictionaryFromBytesFor")]
pub fn load_user_dictionary_from_bytes_for(
    csv: &[u8],
    dictionary: &JsDictionary,
) -> Result<JsUserDictionary, JsValue> {
    let system = &dictionary.inner;
    let builder = DictionaryBuilder::new((*system.metadata).clone())
        .with_system_matrix(&system.connection_cost_matrix);
    let dict = builder
        .build_user_dict_from_reader(csv)
        .map_err(|e| JsValue::from_str(&format!("user_dict csv: {e:?}")))?;
    Ok(JsUserDictionary::new(dict))
}

/// Loads a prebuilt user dictionary (`.bin`) from bytes.
///
/// The bytes are the output of `lindera build --user` (a serialized user
/// dictionary), obtained in JavaScript via `fetch`, a file input, or OPFS.
#[wasm_bindgen(js_name = "loadUserDictionaryBinFromBytes")]
pub fn load_user_dictionary_bin_from_bytes(bytes: &[u8]) -> Result<JsUserDictionary, JsValue> {
    let dict = UserDictionary::load(bytes)
        .map_err(|e| JsValue::from_str(&format!("user_dict bin: {e}")))?;
    Ok(JsUserDictionary::new(dict))
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_load_dictionary() {
        use super::load_dictionary;

        let dict = load_dictionary("embedded://ipadic").unwrap();

        assert!(!dict.name().is_empty());
        assert!(!dict.encoding().is_empty());

        let metadata = dict.metadata();
        assert!(!metadata.name().is_empty());
        assert!(!metadata.encoding().is_empty());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_load_dictionary_invalid_uri() {
        use super::load_dictionary;

        let result = load_dictionary("embedded://nonexistent");

        assert!(result.is_err());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_load_dictionary_from_bytes_invalid_metadata() {
        use super::load_dictionary_from_bytes;

        let result =
            load_dictionary_from_bytes(b"not valid json", &[], &[], &[], &[], &[], &[], &[], &[]);

        assert!(result.is_err());
        let err = result.err().unwrap().as_string().unwrap();
        assert!(
            err.contains("metadata"),
            "error should mention metadata: {err}"
        );
    }

    /// Filesystem URIs and bare paths must fail fast with a pointer at the
    /// bytes API, not with an opaque platform error (#971).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_load_dictionary_fs_uri_fails_fast() {
        use super::load_dictionary;

        for uri in ["file:///path/to/dict", "/path/to/dict", "./relative/dict"] {
            let err = match load_dictionary(uri) {
                Ok(_) => panic!("{uri} must not load on wasm32"),
                Err(err) => err.as_string().unwrap_or_default(),
            };
            assert!(
                err.contains("loadDictionaryFromBytes"),
                "error for {uri} must name the bytes API: {err}"
            );
        }
    }

    /// A user dictionary built from CSV bytes must change tokenization:
    /// `東京スカイツリー` splits under bare IPADIC and becomes a single token
    /// with the user dictionary (#972).
    #[cfg(all(target_arch = "wasm32", feature = "embed-ipadic"))]
    #[wasm_bindgen_test]
    fn test_user_dictionary_from_csv_bytes_changes_tokenization() {
        use super::{load_dictionary, load_user_dictionary_from_bytes};
        use crate::tokenizer::Tokenizer;

        let dict = load_dictionary("embedded://ipadic").unwrap();
        let csv = include_bytes!("../../resources/user_dict/ipadic_simple_userdic.csv");

        let without = Tokenizer::new(&dict, None, None, None).unwrap();
        let baseline = without.tokenize_surfaces("東京スカイツリー").unwrap();
        assert!(
            baseline.len() > 1,
            "fixture assumption: 東京スカイツリー must split under bare IPADIC: {baseline:?}"
        );

        let user_dict = load_user_dictionary_from_bytes(csv, &dict.metadata()).unwrap();
        let with = Tokenizer::new(&dict, None, Some(user_dict.into()), None).unwrap();
        let surfaces = with.tokenize_surfaces("東京スカイツリー").unwrap();
        assert_eq!(
            surfaces,
            vec!["東京スカイツリー".to_string()],
            "the user dictionary must take effect"
        );
    }

    /// A prebuilt `.bin` user dictionary loaded from bytes must work the
    /// same way (#972).
    #[cfg(all(target_arch = "wasm32", feature = "embed-ipadic"))]
    #[wasm_bindgen_test]
    fn test_user_dictionary_bin_from_bytes_changes_tokenization() {
        use super::{load_dictionary, load_user_dictionary_bin_from_bytes};
        use crate::tokenizer::Tokenizer;

        let dict = load_dictionary("embedded://ipadic").unwrap();
        let bin = include_bytes!("../../resources/user_dict/ipadic_simple_userdic.bin");

        let user_dict = load_user_dictionary_bin_from_bytes(bin).unwrap();
        let tokenizer = Tokenizer::new(&dict, None, Some(user_dict.into()), None).unwrap();
        let surfaces = tokenizer.tokenize_surfaces("東京スカイツリー").unwrap();
        assert_eq!(surfaces, vec!["東京スカイツリー".to_string()]);
    }

    /// A CSV built for a loaded system dictionary is checked against its
    /// connection matrix, and a row outside it is reported by its CSV line
    /// and surface (#1193). Line 1 lies inside IPADIC's 1316 x 1316 matrix;
    /// line 2 has the context IDs 60000.
    #[cfg(all(target_arch = "wasm32", feature = "embed-ipadic"))]
    #[wasm_bindgen_test]
    fn test_user_dictionary_from_csv_bytes_for_names_the_csv_line_and_surface() {
        use super::{load_dictionary, load_user_dictionary_from_bytes_for};

        let dict = load_dictionary("embedded://ipadic").unwrap();
        let csv = "東京タワー,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,\
                   東京タワー,トウキョウタワー,トウキョウタワー\n\
                   東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,\
                   東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー\n";

        let err = match load_user_dictionary_from_bytes_for(csv.as_bytes(), &dict) {
            Ok(_) => panic!("a row outside the matrix must be rejected"),
            Err(err) => err.as_string().unwrap_or_default(),
        };
        assert!(
            err.contains("line 2 (surface: '東京スカイツリー')"),
            "the error must name the CSV line and surface: {err}"
        );
        assert!(
            err.contains("left context id 60000 is out of range"),
            "the error must name the context id: {err}"
        );
    }

    /// A CSV inside the matrix loads for the system dictionary, which stays
    /// usable for the tokenizer (#1193).
    #[cfg(all(target_arch = "wasm32", feature = "embed-ipadic"))]
    #[wasm_bindgen_test]
    fn test_user_dictionary_from_csv_bytes_for_changes_tokenization() {
        use super::{load_dictionary, load_user_dictionary_from_bytes_for};
        use crate::tokenizer::Tokenizer;

        let dict = load_dictionary("embedded://ipadic").unwrap();
        let csv = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー\n";

        let user_dict = load_user_dictionary_from_bytes_for(csv.as_bytes(), &dict).unwrap();
        let tokenizer = Tokenizer::new(&dict, None, Some(user_dict.into()), None).unwrap();
        let surfaces = tokenizer.tokenize_surfaces("東京スカイツリー").unwrap();
        assert_eq!(surfaces, vec!["東京スカイツリー".to_string()]);
    }

    /// Malformed CSV (rows shorter than the simple format) must return an
    /// error rather than panic — a panic aborts the module on wasm32 (#972).
    #[cfg(all(target_arch = "wasm32", feature = "embed-ipadic"))]
    #[wasm_bindgen_test]
    fn test_user_dictionary_malformed_csv_returns_error() {
        use super::{load_dictionary, load_user_dictionary_from_bytes};

        let dict = load_dictionary("embedded://ipadic").unwrap();
        let result = load_user_dictionary_from_bytes("東京,1288\n".as_bytes(), &dict.metadata());
        assert!(result.is_err(), "a 2-field row must be rejected, not panic");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_load_dictionary_from_bytes_incomplete_metadata() {
        use super::load_dictionary_from_bytes;

        // Incomplete metadata JSON (missing required fields)
        let metadata = br#"{"name":"test","encoding":"utf-8"}"#;
        let result = load_dictionary_from_bytes(metadata, &[], &[], &[], &[], &[], &[], &[], &[]);

        assert!(result.is_err());
    }

    /// `user_dictionary_from_js` reads a `UserDictionary` through the JS glue
    /// and leaves it usable, and `null`, `undefined` or a left-out argument
    /// mean none (#1238).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_user_dictionary_from_js_reads_a_user_dictionary() {
        use wasm_bindgen::JsValue;

        use super::{load_user_dictionary_bin_from_bytes, user_dictionary_from_js};

        let bin = include_bytes!("../../resources/user_dict/ipadic_simple_userdic.bin");
        let loaded = load_user_dictionary_bin_from_bytes(bin).unwrap();
        let words = loaded.inner.dict.words_data.to_vec();
        let user_dictionary = JsValue::from(loaded);
        for attempt in 0..2 {
            let Some(read) = user_dictionary_from_js(Some(user_dictionary.clone())).unwrap() else {
                panic!("attempt {attempt}: no user dictionary was read");
            };
            assert_eq!(*read.dict.words_data, *words, "attempt {attempt}");
        }

        assert!(user_dictionary_from_js(None).unwrap().is_none());
        assert!(
            user_dictionary_from_js(Some(JsValue::NULL))
                .unwrap()
                .is_none()
        );
        assert!(
            user_dictionary_from_js(Some(JsValue::UNDEFINED))
                .unwrap()
                .is_none()
        );
    }

    /// A value that is not a `UserDictionary` is an error, not none, with the
    /// message the JS glue gives for a class argument. A `Mode` has a
    /// `__copyOf` too, and is rejected all the same (#1238).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_user_dictionary_from_js_rejects_other_values() {
        use wasm_bindgen::{JsCast, JsValue};

        use super::{load_dictionary, user_dictionary_from_js};
        use crate::metadata::JsMetadata;
        use crate::mode::JsMode;

        let without_constructor: js_sys::Object =
            js_sys::Object::create(&JsValue::NULL.unchecked_into());
        for value in [
            JsValue::from_str("user_dictionary"),
            JsValue::from_f64(0.0),
            JsValue::TRUE,
            js_sys::Object::new().into(),
            without_constructor.into(),
            JsValue::from(JsMode::normal()),
            JsValue::from(JsMetadata::create_default()),
            JsValue::from(load_dictionary("embedded://ipadic").unwrap()),
        ] {
            let shown = format!("{value:?}");
            let message = match user_dictionary_from_js(Some(value)) {
                Ok(read) => panic!("{shown} was read: {}", read.is_some()),
                Err(error) => error
                    .dyn_into::<js_sys::Error>()
                    .map(|error| String::from(error.message()))
                    .unwrap_or_else(|error| format!("not an Error: {error:?}")),
            };
            assert_eq!(message, "expected instance of UserDictionary", "{shown}");
        }
    }

    /// `loadUserDictionaryFromBytes` and its alias borrow the metadata, so
    /// one `Metadata` can be passed to several calls (#1238). A test cannot
    /// call a free function through the JS glue; the glue leaves an argument
    /// usable because the function takes a reference, which these calls
    /// need to compile.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_load_user_dictionary_from_bytes_borrows_the_metadata() {
        use super::{load_dictionary, load_user_dictionary_from_bytes};

        let metadata = load_dictionary("embedded://ipadic").unwrap().metadata();
        let csv = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー\n".as_bytes();
        for attempt in 0..2 {
            assert!(
                load_user_dictionary_from_bytes(csv, &metadata).is_ok(),
                "attempt {attempt}"
            );
            assert!(
                crate::py_load_user_dictionary_from_bytes(csv, &metadata).is_ok(),
                "alias, attempt {attempt}"
            );
        }
    }

    /// A copy made by `__copyOf` shares the user dictionary instead of
    /// copying its automaton and data (#1238).
    #[test]
    fn test_copy_of_shares_the_user_dictionary() {
        use std::rc::Rc;

        use super::{JsUserDictionary, load_user_dictionary_bin_from_bytes};

        let bin = include_bytes!("../../resources/user_dict/ipadic_simple_userdic.bin");
        let user_dictionary = load_user_dictionary_bin_from_bytes(bin).unwrap();
        let copy = JsUserDictionary::copy_of(&user_dictionary);
        assert!(Rc::ptr_eq(&copy.inner, &user_dictionary.inner));
    }
}
