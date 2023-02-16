use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct FilteredToken {
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

    /// Detailes about the token.
    /// It contains metadata for tokens, such as part-of-speech information.
    pub details: Vec<String>,
}
