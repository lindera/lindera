use lindera::token::{BunsetsuRole, Token, TokenDependency};

/// FFI-independent view of a [`lindera::token::TokenDependency`]: the
/// bunsetsu of a token and what that bunsetsu depends on, which the token
/// filter `japanese_dependency` writes.
#[derive(Debug, Clone, PartialEq)]
pub struct DependencyView {
    /// The number of the sentence in the text, from 0.
    pub sentence: u32,
    /// The number of the bunsetsu of the token in its sentence, from 0.
    pub bunsetsu: u32,
    /// The number of the bunsetsu that one depends on, or `None` for the
    /// last bunsetsu of a sentence.
    pub head: Option<u32>,
    /// What the token is in its bunsetsu.
    pub role: BunsetsuRole,
    /// The score of the decision that attached the bunsetsu to its head.
    pub score: f64,
}

impl DependencyView {
    /// Copies the dependency of a token.
    ///
    /// # Arguments
    ///
    /// * `dependency` - The dependency of a `lindera` token.
    ///
    /// # Returns
    ///
    /// The view.
    pub fn from_dependency(dependency: &TokenDependency) -> Self {
        Self {
            sentence: dependency.sentence,
            bunsetsu: dependency.bunsetsu,
            head: dependency.head,
            role: dependency.role,
            score: dependency.score,
        }
    }

    /// The name of the role, as every binding gives it.
    ///
    /// # Returns
    ///
    /// `head`, `function`, `head_function` or `other`.
    pub fn role(&self) -> &'static str {
        self.role.as_str()
    }
}

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
    /// The dependency of the token; `None` unless the token filter
    /// `japanese_dependency` wrote one.
    pub dependency: Option<DependencyView>,
}

impl TokenView {
    /// Extracts the binding-facing data from a `lindera` token.
    pub fn from_token(mut token: Token) -> Self {
        // details_iter reads the details in one pass, without the
        // intermediate Vec<&str> that details() collects on every call or
        // the per-token cache it loads (#1399); its exact length lets the
        // collect below allocate once.
        let details = token.details_iter().map(|s| s.to_string()).collect();
        let dependency = token.dependency().map(DependencyView::from_dependency);

        Self {
            surface: token.surface.to_string(),
            byte_start: token.byte_start,
            byte_end: token.byte_end,
            position: token.position,
            word_id: token.word_id.id(),
            is_unknown: token.word_id.is_unknown(),
            details,
            dependency,
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
    /// # Arguments
    ///
    /// * `token` - The token to consume; a borrowed surface is copied
    ///   once, an owned surface is moved.
    ///
    /// # Returns
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
