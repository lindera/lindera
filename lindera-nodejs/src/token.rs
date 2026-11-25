//! Token representation for morphological analysis results.
//!
//! Tokens cross into JavaScript as plain objects rather than class
//! instances. A napi class instance releases its native memory through a
//! finalizer that napi defers to the event loop, so a synchronous loop that
//! tokenizes without yielding accumulates native memory even under forced
//! GC. Plain objects are owned entirely by the JS heap, so the same loop
//! stays flat, and they survive `JSON.stringify` / `structuredClone` /
//! worker transfer without conversion (#922, #930).
//!
//! `Token` and `NbestResult` give their types to `index.d.ts`; their
//! objects are made by `crate::token_objects`, the same as the conversion
//! napi-rs derives from them (#1409).

use napi::bindgen_prelude::{Either, Null};

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

/// The bunsetsu of a token and what that bunsetsu depends on: what the
/// token filter `japanese_dependency` writes on a token.
///
/// The bunsetsu of a sentence are numbered from 0, and each of them but the
/// last depends on a later one, its head.
#[napi(object)]
pub struct TokenDependency {
    /// The number of the sentence in the text, from 0.
    pub sentence: u32,
    /// The number of the bunsetsu of the token in its sentence, from 0.
    pub bunsetsu: u32,
    /// The number of the bunsetsu that the bunsetsu of the token depends
    /// on, or `null` for the last bunsetsu of a sentence.
    pub head: Either<u32, Null>,
    /// What the token is in its bunsetsu.
    #[napi(ts_type = "'head' | 'function' | 'head_function' | 'other'")]
    pub role: String,
    /// The score of the decision that attached the bunsetsu to its head; 0
    /// where there was nothing to decide.
    pub score: f64,
}

impl TokenDependency {
    /// Creates a `TokenDependency` from a lindera-binding `DependencyView`.
    ///
    /// # Arguments
    ///
    /// * `view` - The dependency of a token view.
    ///
    /// # Returns
    ///
    /// A new `TokenDependency` instance.
    pub fn from_view(view: lindera_binding::DependencyView) -> Self {
        Self {
            sentence: view.sentence,
            bunsetsu: view.bunsetsu,
            head: match view.head {
                Some(head) => Either::A(head),
                None => Either::B(Null),
            },
            role: view.role().to_string(),
            score: view.score,
        }
    }
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
    /// The bunsetsu of the token and its head. The property is there only
    /// when the token filter `japanese_dependency` ran.
    pub dependency: Option<TokenDependency>,
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
            dependency: view.dependency.map(TokenDependency::from_view),
        }
    }
}

/// Makes the surface-only views of a batch into surface strings, text by
/// text. The synchronous `tokenizeSurfacesBatch` and the parts of its
/// `Promise` version both convert through it.
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

/// Makes the surfaces of a batch at once, as `tokenizeSurfacesBatch` gives
/// them: the conversion of a batch that fits in one part
/// (`crate::batch_parts`).
///
/// # Arguments
///
/// * `env` - The environment of the current call.
/// * `batch` - The views of each text.
///
/// # Returns
///
/// The array of the surface strings of each text.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn surfaces_value(
    env: napi::sys::napi_env,
    batch: Vec<Vec<lindera_binding::token::SurfaceView>>,
) -> napi::Result<napi::sys::napi_value> {
    // SAFETY: the caller guarantees `env`.
    unsafe { napi::bindgen_prelude::ToNapiValue::to_napi_value(env, surfaces_from_views(batch)) }
}
