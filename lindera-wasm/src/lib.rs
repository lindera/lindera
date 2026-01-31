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
pub use crate::metadata::{JsCompressionAlgorithm as CompressionAlgorithm, JsMetadata as Metadata};
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
