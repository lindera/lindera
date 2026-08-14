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
        // details_iter avoids the intermediate Vec<&str> that details()
        // collects on every call.
        let details = token.details_iter().map(|s| s.to_string()).collect();

        Self {
            surface: token.surface.to_string(),
            byte_start: token.byte_start,
            byte_end: token.byte_end,
            position: token.position,
            word_id: token.word_id.id(),
            is_unknown: token.word_id.is_unknown(),
            details,
        }
    }
}

/// Surface-only view of a [`lindera::token::Token`], for callers that do
/// not need morphological details (wakati-style use).
///
/// Unlike [`TokenView::from_token`], constructing this never touches the
/// dictionary's word details, skipping their materialization entirely
/// (roughly one allocation per token instead of ~14 for IPADIC). Byte
/// offsets are carried here so bindings can expose them later without a
/// breaking change; the current language APIs return only the surfaces.
#[derive(Debug, Clone)]
pub struct SurfaceView {
    /// The token's surface form.
    pub surface: String,
    /// Starting byte position in the original (pre-filter) text.
    pub byte_start: usize,
    /// Ending byte position (exclusive) in the original (pre-filter) text.
    pub byte_end: usize,
}

impl SurfaceView {
    /// Extracts the surface data from a `lindera` token without loading
    /// details.
    ///
    /// # 引数
    ///
    /// * `token` - The token to consume; a borrowed surface is copied
    ///   once, an owned surface is moved.
    ///
    /// # 戻り値
    ///
    /// The surface-only view.
    pub fn from_token(token: Token) -> Self {
        Self {
            surface: token.surface.into_owned(),
            byte_start: token.byte_start,
            byte_end: token.byte_end,
        }
    }
}
