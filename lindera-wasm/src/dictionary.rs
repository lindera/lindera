use std::path::Path;

use wasm_bindgen::prelude::*;

use lindera::dictionary::{
    Dictionary, DictionaryBuilder, UserDictionary, load_dictionary as lindera_load_dictionary,
    load_user_dictionary as lindera_load_user_dictionary,
};
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;

use crate::metadata::JsMetadata;

/// Decompresses dictionary data if it is in lindera's compressed format.
///
/// Dictionary files built with the `compress` feature are wrapped in a
/// `CompressedData` envelope serialized with rkyv. This function attempts
/// to deserialize and decompress the data; if the data is not compressed,
/// the original bytes are returned as-is.
///
/// # Arguments
///
/// * `data` - Raw bytes that may or may not be compressed.
///
/// # Returns
///
/// The decompressed bytes as a `Vec<u8>`.
fn try_decompress(data: &[u8]) -> Vec<u8> {
    use lindera_dictionary::decompress::{CompressedData, decompress};

    let mut aligned = rkyv::util::AlignedVec::<16>::new();
    aligned.extend_from_slice(data);
    match rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned) {
        Ok(compressed_data) => match decompress(compressed_data) {
            Ok(decompressed) => decompressed,
            Err(_) => data.to_vec(),
        },
        Err(_) => data.to_vec(),
    }
}

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

/// Loads a dictionary from raw byte arrays.
///
/// This function constructs a `Dictionary` directly from the binary data
/// of each dictionary component file, without requiring filesystem access.
/// This is useful for loading dictionaries from OPFS or other browser storage.
///
/// # Arguments
///
/// * `metadata` - The contents of `metadata.json`
/// * `dict_da` - The contents of `dict.da` (Double-Array Trie)
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
#[wasm_bindgen(js_name = "loadDictionaryFromBytes")]
#[allow(clippy::too_many_arguments)]
pub fn load_dictionary_from_bytes(
    metadata: &[u8],
    dict_da: &[u8],
    dict_vals: &[u8],
    dict_words_idx: &[u8],
    dict_words: &[u8],
    matrix_mtx: &[u8],
    char_def: &[u8],
    unk: &[u8],
) -> Result<JsDictionary, JsValue> {
    let meta =
        Metadata::load(metadata).map_err(|e| JsValue::from_str(&format!("metadata: {e}")))?;

    // Decompress dictionary data if compressed (lindera compress feature)
    let da_data = try_decompress(dict_da);
    let vals_data = try_decompress(dict_vals);
    let words_idx_data = try_decompress(dict_words_idx);
    let words_data = try_decompress(dict_words);
    let conn_data = try_decompress(matrix_mtx);
    let char_def_data = try_decompress(char_def);
    let unk_data = try_decompress(unk);

    let dict = Dictionary {
        prefix_dictionary: PrefixDictionary::load(
            da_data,
            vals_data,
            words_idx_data,
            words_data,
            true,
        ),
        connection_cost_matrix: ConnectionCostMatrix::load(conn_data),
        character_definition: CharacterDefinition::load(&char_def_data)
            .map_err(|e| JsValue::from_str(&format!("char_def: {e}")))?,
        unknown_dictionary: UnknownDictionary::load(&unk_data)
            .map_err(|e| JsValue::from_str(&format!("unk: {e}")))?,
        metadata: meta,
    };

    Ok(JsDictionary { inner: dict })
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
            load_dictionary_from_bytes(b"not valid json", &[], &[], &[], &[], &[], &[], &[]);

        assert!(result.is_err());
        let err = result.err().unwrap().as_string().unwrap();
        assert!(
            err.contains("metadata"),
            "error should mention metadata: {err}"
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_load_dictionary_from_bytes_invalid_char_def() {
        use super::load_dictionary_from_bytes;

        // Valid minimal metadata JSON, but invalid char_def binary
        let metadata = br#"{"name":"test","encoding":"utf-8"}"#;
        let result = load_dictionary_from_bytes(
            metadata,
            &[],
            &[],
            &[],
            &[],
            &[],
            b"invalid char_def data",
            &[],
        );

        assert!(result.is_err());
        let err = result.err().unwrap().as_string().unwrap();
        assert!(
            err.contains("char_def"),
            "error should mention char_def: {err}"
        );
    }
}
