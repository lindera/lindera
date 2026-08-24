use std::sync::Arc;

use wasm_bindgen::prelude::*;

use lindera::dictionary::{
    Dictionary, DictionaryBuilder, UserDictionary, load_dictionary as lindera_load_dictionary,
};
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;

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
/// # Breaking change (dictionary format version 2)
///
/// Dictionaries built before Lindera v6 shipped a byte-wise automaton as
/// `dict.da`; that argument was replaced by `dict_trie` + `dict_vals_idx`.
/// Rebuild the dictionary or download a matching prebuilt one.
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
#[wasm_bindgen(js_name = "loadUserDictionaryFromBytes")]
pub fn load_user_dictionary_from_bytes(
    csv: &[u8],
    metadata: JsMetadata,
) -> Result<JsUserDictionary, JsValue> {
    let meta: Metadata = metadata.into();
    let builder = DictionaryBuilder::new(meta);
    let dict = builder
        .build_user_dict_from_reader(csv)
        .map_err(|e| JsValue::from_str(&format!("user_dict csv: {e:?}")))?;
    Ok(JsUserDictionary { inner: dict })
}

/// Loads a prebuilt user dictionary (`.bin`) from bytes.
///
/// The bytes are the output of `lindera build --user` (a serialized user
/// dictionary), obtained in JavaScript via `fetch`, a file input, or OPFS.
#[wasm_bindgen(js_name = "loadUserDictionaryBinFromBytes")]
pub fn load_user_dictionary_bin_from_bytes(bytes: &[u8]) -> Result<JsUserDictionary, JsValue> {
    let dict = UserDictionary::load(bytes)
        .map_err(|e| JsValue::from_str(&format!("user_dict bin: {e}")))?;
    Ok(JsUserDictionary { inner: dict })
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

        let without = Tokenizer::new(dict.clone(), None, None).unwrap();
        let baseline = without.tokenize_surfaces("東京スカイツリー").unwrap();
        assert!(
            baseline.len() > 1,
            "fixture assumption: 東京スカイツリー must split under bare IPADIC: {baseline:?}"
        );

        let user_dict = load_user_dictionary_from_bytes(csv, dict.metadata()).unwrap();
        let with = Tokenizer::new(dict, None, Some(user_dict)).unwrap();
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
        let tokenizer = Tokenizer::new(dict, None, Some(user_dict)).unwrap();
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
        let result = load_user_dictionary_from_bytes("東京,1288\n".as_bytes(), dict.metadata());
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
}
