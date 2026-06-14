use lindera::token::Token;

/// FFI-independent view of a [`lindera::token::Token`].
///
/// Holds the pure data each binding needs to expose, so the extraction logic
/// (loading details, reading the word id) lives in one place. Each binding
/// then maps these fields onto its own FFI token type.
#[derive(Debug, Clone)]
pub struct TokenView {
    pub surface: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub position: usize,
    pub word_id: u32,
    pub is_unknown: bool,
    pub details: Vec<String>,
}

impl TokenView {
    /// Extracts the binding-facing data from a `lindera` token.
    pub fn from_token(mut token: Token) -> Self {
        let details = token.details().iter().map(|s| s.to_string()).collect();

        Self {
            surface: token.surface.to_string(),
            byte_start: token.byte_start,
            byte_end: token.byte_end,
            position: token.position,
            word_id: token.word_id.id,
            is_unknown: token.word_id.is_unknown(),
            details,
        }
    }
}
