//! The tasks of the batch methods that return a `Promise`
//! (`tokenizeBatchAsync`, `tokenizeSurfacesBatchAsync` and
//! `tokenizeNbestBatchAsync`, #1406).
//!
//! napi runs a task's `compute` on a thread of libuv's pool and its
//! `resolve` on the JavaScript thread, so the texts are tokenized while the
//! event loop runs, and the token objects are made afterwards on the
//! JavaScript thread. A task owns a clone of the tokenizer's
//! `Arc<CoreTokenizer>`, as the `Tokenizer` object may be collected while
//! the task runs.
//!
//! The arguments are checked on the JavaScript thread, when the task is made;
//! an argument out of its range is kept in the task, and `compute` returns
//! its error, which rejects the `Promise`.
//!
//! Each task is a type of its own, with an `impl Task` of its own: napi's
//! type generation finds the `JsValue` of a task by the name of its type,
//! and gives a generic task `Promise<unknown>` in `index.d.ts`.

use std::num::NonZeroUsize;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use napi::bindgen_prelude::panic_to_error;
use napi::{Env, Task};

use lindera_binding::token::SurfaceView;
use lindera_binding::tokenizer::NbestViews;
use lindera_binding::{CoreTokenizer, TokenView};

use crate::error::to_napi_error;
use crate::token::{NbestResult, Token, nbest_from_views, surfaces_from_views, tokens_from_views};

/// Runs the work of a task's `compute`, making a panic in it the error that
/// `#[napi(catch_unwind)]` makes of a panic (`GenericFailure`, with the
/// panic's message).
///
/// napi calls `compute` without catching a panic, so a panic would unwind
/// across an `extern "C"` function and abort Node.js; a batch resumes the
/// first panic of its texts on the thread that runs it, which is libuv's
/// thread here. Every task's `compute` goes through this function.
///
/// # Arguments
///
/// * `work` - The work, run on the calling thread.
///
/// # Returns
///
/// What `work` returns, or the error of its panic.
pub(crate) fn compute_batch<R>(work: impl FnOnce() -> napi::Result<R>) -> napi::Result<R> {
    catch_unwind(AssertUnwindSafe(work)).map_err(panic_to_error)?
}

/// The checked arguments of an N-best batch, as `tokenizeNbestBatch` takes
/// them.
pub(crate) struct NbestArgs {
    /// Number of N-best results per text.
    pub(crate) n: usize,
    /// Deduplicate results with identical word boundaries.
    pub(crate) unique: bool,
    /// Maximum cost difference from the best path of each text.
    pub(crate) cost_threshold: Option<i64>,
    /// The most threads, or `None` for the default.
    pub(crate) max_threads: Option<NonZeroUsize>,
}

/// The task of `tokenizeBatchAsync`.
pub struct TokenizeBatchTask {
    /// The tokenizer, shared with the `Tokenizer` object.
    tokenizer: Arc<CoreTokenizer>,
    /// The texts, taken by `compute`.
    texts: Vec<String>,
    /// The checked `maxThreads`, or the error that rejects the `Promise`;
    /// taken by `compute`.
    max_threads: Option<napi::Result<Option<NonZeroUsize>>>,
}

impl TokenizeBatchTask {
    /// Makes the task.
    ///
    /// # Arguments
    ///
    /// * `tokenizer` - The tokenizer.
    /// * `texts` - The texts.
    /// * `max_threads` - The checked `maxThreads`, or its error.
    ///
    /// # Returns
    ///
    /// The task.
    pub(crate) fn new(
        tokenizer: Arc<CoreTokenizer>,
        texts: Vec<String>,
        max_threads: napi::Result<Option<NonZeroUsize>>,
    ) -> Self {
        Self {
            tokenizer,
            texts,
            max_threads: Some(max_threads),
        }
    }
}

#[napi]
impl Task for TokenizeBatchTask {
    type Output = Vec<Vec<TokenView>>;
    type JsValue = Vec<Vec<Token>>;

    /// Tokenizes the texts, on a thread of libuv's pool, as `tokenizeBatch`
    /// does, or returns the error of an argument out of its range.
    fn compute(&mut self) -> napi::Result<Self::Output> {
        let max_threads = taken(&mut self.max_threads)?;
        let texts = std::mem::take(&mut self.texts);
        let tokenizer = &self.tokenizer;
        compute_batch(|| {
            tokenizer
                .tokenize_batch(&texts, max_threads)
                .map_err(to_napi_error)
        })
    }

    /// Makes the token objects, on the JavaScript thread.
    fn resolve(&mut self, _env: Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(tokens_from_views(output))
    }
}

/// The task of `tokenizeSurfacesBatchAsync`.
pub struct TokenizeSurfacesBatchTask {
    /// The tokenizer, shared with the `Tokenizer` object.
    tokenizer: Arc<CoreTokenizer>,
    /// The texts, taken by `compute`.
    texts: Vec<String>,
    /// The checked `maxThreads`, or the error that rejects the `Promise`;
    /// taken by `compute`.
    max_threads: Option<napi::Result<Option<NonZeroUsize>>>,
}

impl TokenizeSurfacesBatchTask {
    /// Makes the task.
    ///
    /// # Arguments
    ///
    /// * `tokenizer` - The tokenizer.
    /// * `texts` - The texts.
    /// * `max_threads` - The checked `maxThreads`, or its error.
    ///
    /// # Returns
    ///
    /// The task.
    pub(crate) fn new(
        tokenizer: Arc<CoreTokenizer>,
        texts: Vec<String>,
        max_threads: napi::Result<Option<NonZeroUsize>>,
    ) -> Self {
        Self {
            tokenizer,
            texts,
            max_threads: Some(max_threads),
        }
    }
}

#[napi]
impl Task for TokenizeSurfacesBatchTask {
    type Output = Vec<Vec<SurfaceView>>;
    type JsValue = Vec<Vec<String>>;

    /// Tokenizes the texts, on a thread of libuv's pool, as
    /// `tokenizeSurfacesBatch` does, or returns the error of an argument out
    /// of its range.
    fn compute(&mut self) -> napi::Result<Self::Output> {
        let max_threads = taken(&mut self.max_threads)?;
        let texts = std::mem::take(&mut self.texts);
        let tokenizer = &self.tokenizer;
        compute_batch(|| {
            tokenizer
                .tokenize_surfaces_batch(&texts, max_threads)
                .map_err(to_napi_error)
        })
    }

    /// Makes the surface strings, on the JavaScript thread.
    fn resolve(&mut self, _env: Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(surfaces_from_views(output))
    }
}

/// The task of `tokenizeNbestBatchAsync`.
pub struct TokenizeNbestBatchTask {
    /// The tokenizer, shared with the `Tokenizer` object.
    tokenizer: Arc<CoreTokenizer>,
    /// The texts, taken by `compute`.
    texts: Vec<String>,
    /// The checked arguments, or the error that rejects the `Promise`; taken
    /// by `compute`.
    args: Option<napi::Result<NbestArgs>>,
}

impl TokenizeNbestBatchTask {
    /// Makes the task.
    ///
    /// # Arguments
    ///
    /// * `tokenizer` - The tokenizer.
    /// * `texts` - The texts.
    /// * `args` - The checked arguments, or the error of one of them.
    ///
    /// # Returns
    ///
    /// The task.
    pub(crate) fn new(
        tokenizer: Arc<CoreTokenizer>,
        texts: Vec<String>,
        args: napi::Result<NbestArgs>,
    ) -> Self {
        Self {
            tokenizer,
            texts,
            args: Some(args),
        }
    }
}

#[napi]
impl Task for TokenizeNbestBatchTask {
    type Output = Vec<NbestViews>;
    type JsValue = Vec<Vec<NbestResult>>;

    /// Tokenizes the texts, on a thread of libuv's pool, as
    /// `tokenizeNbestBatch` does, or returns the error of an argument out of
    /// its range.
    fn compute(&mut self) -> napi::Result<Self::Output> {
        let args = taken(&mut self.args)?;
        let texts = std::mem::take(&mut self.texts);
        let tokenizer = &self.tokenizer;
        compute_batch(|| {
            tokenizer
                .tokenize_nbest_batch(
                    &texts,
                    args.n,
                    args.unique,
                    args.cost_threshold,
                    args.max_threads,
                )
                .map_err(to_napi_error)
        })
    }

    /// Makes the `NbestResult` objects, on the JavaScript thread.
    fn resolve(&mut self, _env: Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(nbest_from_views(output))
    }
}

/// Takes the checked arguments of a task, which `compute` takes once.
///
/// # Arguments
///
/// * `slot` - The arguments, or the error of one of them.
///
/// # Returns
///
/// The arguments, the error of one of them, or an error when `compute` runs
/// a second time (napi runs it once).
fn taken<T>(slot: &mut Option<napi::Result<T>>) -> napi::Result<T> {
    slot.take().unwrap_or_else(|| {
        Err(napi::Error::new(
            napi::Status::GenericFailure,
            "the task has already run",
        ))
    })
}
