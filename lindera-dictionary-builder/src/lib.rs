//! This library is used to build custom [lindera](https://github.com/lindera-morphology/lindera) dictionary files.
//!
//! Normally, you don't need to use this library.
//! Instead, use one of the pre-built dictionaries (e.g. ipadic, unidic, ...)
//! by enabling a feature flag for the lindera-tokenizer crate.

pub mod chardef;
pub mod cost_matrix;
pub mod dict;
pub mod unk;
pub mod user_dict;
pub mod utils;

pub use chardef::CharDefBuilderOptions;
pub use cost_matrix::CostMatrixBuilderOptions;
pub use dict::DictBuilderOptions;
pub use unk::UnkBuilderOptions;
pub use user_dict::{build_user_dictionary, UserDictBuilderOptions};
