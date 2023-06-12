use serde::Serialize;

use lindera_core::word_entry::WordId;

#[derive(Serialize, Clone)]
pub struct Token {
    /// Text content of the token.
    pub text: String,

    /// Starting position of the token in bytes.
    pub byte_start: usize,

    /// Ending position of the token in bytes.
    pub byte_end: usize,

    /// Position, expressed in number of tokens.
    pub position: usize,

    /// The length expressed in term of number of original tokens.
    pub position_length: usize,

    /// The ID of the word and a flag to indicate whether the word is registered in the dictionary.
    pub word_id: WordId,

    /// Detailes about the token.
    /// It contains metadata for tokens, such as part-of-speech information.
    pub details: Vec<String>,
}
