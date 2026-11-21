//! Token representation for morphological analysis results.
//!
//! Tokens cross into JavaScript as plain objects rather than class
//! instances. A napi class instance releases its native memory through a
//! finalizer that napi defers to the event loop, so a synchronous loop that
//! tokenizes without yielding accumulates native memory even under forced
//! GC. Plain objects are owned entirely by the JS heap, so the same loop
//! stays flat, and they survive `JSON.stringify` / `structuredClone` /
//! worker transfer without conversion (#922, #930).

/// N-best tokenization result.
///
/// Contains a list of tokens and their total path cost.
#[napi(object)]
pub struct NbestResult {
    /// Tokens in this result.
    pub tokens: Vec<Token>,
    /// Total path cost of this tokenization.
    pub cost: i64,
}

/// A morphological token.
///
/// Represents a single token from morphological analysis with its surface
/// form, position information, and morphological details.
#[napi(object)]
pub struct Token {
    /// Surface form of the token.
    pub surface: String,
    /// Start byte position in the original text.
    pub byte_start: u32,
    /// End byte position in the original text.
    pub byte_end: u32,
    /// Position index of the token.
    pub position: u32,
    /// Word ID in the dictionary.
    pub word_id: u32,
    /// Whether this token is an unknown word.
    pub is_unknown: bool,
    /// Morphological details of the token.
    pub details: Vec<String>,
}

impl Token {
    /// Creates a Token from a lindera-binding `TokenView`.
    ///
    /// # Arguments
    ///
    /// * `view` - The token view produced by the lindera-binding tokenizer.
    ///
    /// # Returns
    ///
    /// A new Token instance.
    pub fn from_view(view: lindera_binding::TokenView) -> Self {
        Self {
            surface: view.surface,
            byte_start: view.byte_start as u32,
            byte_end: view.byte_end as u32,
            position: view.position as u32,
            word_id: view.word_id,
            is_unknown: view.is_unknown,
            details: view.details,
        }
    }
}

/// Makes the token views of a batch into token objects, text by text.
///
/// The synchronous batch methods and the tasks of their `Promise` versions
/// (`crate::batch_task`) all convert through this function and the two
/// below, so that the loop over the tokens has one copy: a second copy of
/// such a loop once stopped the compiler from inlining the conversion of a
/// token into the loop of the single-text methods (#1407).
///
/// # Arguments
///
/// * `batch` - The views of each text, in the order of the texts.
///
/// # Returns
///
/// The token objects of each text, in the same order.
pub(crate) fn tokens_from_views(batch: Vec<Vec<lindera_binding::TokenView>>) -> Vec<Vec<Token>> {
    batch
        .into_iter()
        .map(|views| views.into_iter().map(Token::from_view).collect())
        .collect()
}

/// Makes the surface-only views of a batch into surface strings, text by
/// text, as [`tokens_from_views`] does for tokens.
///
/// # Arguments
///
/// * `batch` - The views of each text, in the order of the texts.
///
/// # Returns
///
/// The surface strings of each text, in the same order.
pub(crate) fn surfaces_from_views(
    batch: Vec<Vec<lindera_binding::token::SurfaceView>>,
) -> Vec<Vec<String>> {
    batch
        .into_iter()
        .map(|views| views.into_iter().map(|view| view.surface).collect())
        .collect()
}

/// Makes the N-best results of a batch into `NbestResult` objects, text by
/// text, as [`tokens_from_views`] does for tokens.
///
/// # Arguments
///
/// * `batch` - The `(views, cost)` pairs of each text, in the order of the
///   texts.
///
/// # Returns
///
/// The N-best results of each text, in the same order.
pub(crate) fn nbest_from_views(
    batch: Vec<lindera_binding::tokenizer::NbestViews>,
) -> Vec<Vec<NbestResult>> {
    batch
        .into_iter()
        .map(|results| {
            results
                .into_iter()
                .map(|(views, cost)| NbestResult {
                    tokens: views.into_iter().map(Token::from_view).collect(),
                    cost,
                })
                .collect()
        })
        .collect()
}
