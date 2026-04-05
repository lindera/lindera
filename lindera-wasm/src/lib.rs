//! # lindera-wasm
//!
//! WebAssembly bindings for [Lindera](https://github.com/lindera/lindera), a morphological analysis library.
//!
//! This crate provides WASM bindings that enable Japanese, Korean, and Chinese text tokenization
//! in web browsers and bundler environments.
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
//!     builder.set_dictionary("embedded://ipadic");
//!     builder.set_mode("normal");
//!
//!     const tokenizer = builder.build();
//!     const tokens = tokenizer.tokenize("関西国際空港");
//!     console.log(tokens);
//! });
//! ```
//! ```

pub mod character_filter;
pub mod dictionary;
pub mod error;
pub mod metadata;
pub mod mode;
pub mod schema;
pub mod segmenter;
pub mod token;
pub mod token_filter;
pub mod tokenizer;

use wasm_bindgen::prelude::*;

pub use crate::dictionary::{JsDictionary as Dictionary, JsUserDictionary as UserDictionary};
pub use crate::error::JsLinderaError as LinderaError;
pub use crate::metadata::JsMetadata as Metadata;
pub use crate::mode::{JsMode as Mode, JsPenalty as Penalty};
pub use crate::schema::{
    JsFieldDefinition as FieldDefinition, JsFieldType as FieldType, JsSchema as Schema,
};
pub use crate::segmenter::JsSegmenter as Segmenter;
pub use crate::token::JsToken as Token;
pub use crate::tokenizer::{Tokenizer, TokenizerBuilder};

// Top-level function aliases for consistency with Python API
#[wasm_bindgen(js_name = "load_dictionary")]
pub fn py_load_dictionary(uri: &str) -> Result<Dictionary, JsValue> {
    crate::dictionary::load_dictionary(uri)
}

#[wasm_bindgen(js_name = "load_dictionary_from_bytes")]
#[allow(clippy::too_many_arguments)]
pub fn py_load_dictionary_from_bytes(
    metadata: &[u8],
    dict_da: &[u8],
    dict_vals: &[u8],
    dict_words_idx: &[u8],
    dict_words: &[u8],
    matrix_mtx: &[u8],
    char_def: &[u8],
    unk: &[u8],
) -> Result<Dictionary, JsValue> {
    crate::dictionary::load_dictionary_from_bytes(
        metadata,
        dict_da,
        dict_vals,
        dict_words_idx,
        dict_words,
        matrix_mtx,
        char_def,
        unk,
    )
}

#[wasm_bindgen(js_name = "load_user_dictionary")]
pub fn py_load_user_dictionary(uri: &str, metadata: Metadata) -> Result<UserDictionary, JsValue> {
    crate::dictionary::load_user_dictionary(uri, metadata)
}

#[wasm_bindgen(js_name = "build_dictionary")]
pub fn py_build_dictionary(
    input_dir: &str,
    output_dir: &str,
    metadata: Metadata,
) -> Result<(), JsValue> {
    crate::dictionary::build_dictionary(input_dir, output_dir, metadata)
}

#[wasm_bindgen(js_name = "build_user_dictionary")]
pub fn py_build_user_dictionary(
    input_file: &str,
    output_dir: &str,
    metadata: Option<Metadata>,
) -> Result<(), JsValue> {
    crate::dictionary::build_user_dictionary(input_file, output_dir, metadata)
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of the lindera-wasm package.
#[wasm_bindgen]
pub fn version() -> String {
    VERSION.to_string()
}

/// Gets the version of the lindera-wasm library.
/// Backward compatibility alias for version().
#[wasm_bindgen(js_name = "getVersion")]
pub fn get_version() -> String {
    version()
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_version() {
        let v = super::version();
        assert!(!v.is_empty());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_get_version() {
        let v1 = super::version();
        let v2 = super::get_version();
        assert_eq!(v1, v2);
    }
}
