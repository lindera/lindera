//! Shared tokenizer build-flow orchestration for the bindings.
//!
//! Each binding's tokenizer wrapper reimplements the same flow: configure a
//! [`lindera_analysis::tokenizer::TokenizerBuilder`], build a
//! [`lindera_analysis::tokenizer::Tokenizer`], and convert the resulting tokens. This
//! module collects that orchestration into [`CoreTokenizerBuilder`] and
//! [`CoreTokenizer`], leaving each binding to do only its FFI-value conversion
//! (`serde_json::Value` ⇔ the host language's argument type) and a thin wrapper.

use std::path::Path;
use std::str::FromStr;
use std::sync::{Mutex, MutexGuard};

use serde_json::Value;

use lindera::dictionary::{Dictionary, UserDictionary};
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::tokenizer::{Tokenizer, TokenizerBuilder};
use lindera_analysis::worker::AnalysisWorker;

use crate::error::CoreResult;
use crate::token::TokenView;

/// Builder that orchestrates tokenizer configuration on behalf of the bindings.
///
/// Wraps [`lindera_analysis::tokenizer::TokenizerBuilder`]; filter arguments are passed
/// as [`serde_json::Value`] so the FFI-specific value conversion stays in each
/// binding.
pub struct CoreTokenizerBuilder {
    /// The backing lindera builder.
    inner: TokenizerBuilder,
}

impl CoreTokenizerBuilder {
    /// Creates a builder with the default (empty) configuration.
    pub fn new() -> CoreResult<Self> {
        Ok(Self {
            inner: TokenizerBuilder::new()?,
        })
    }

    /// Creates a builder from a YAML configuration file.
    pub fn from_file(file_path: &Path) -> CoreResult<Self> {
        Ok(Self {
            inner: TokenizerBuilder::from_file(file_path)?,
        })
    }

    /// Sets the segmenter mode from a string (`"normal"` or `"decompose"`).
    pub fn set_mode(&mut self, mode: &str) -> CoreResult<&mut Self> {
        let mode = Mode::from_str(mode)?;
        self.inner.set_segmenter_mode(&mode);
        Ok(self)
    }

    /// Sets the segmenter dictionary URI / path.
    pub fn set_dictionary(&mut self, uri: &str) -> &mut Self {
        self.inner.set_segmenter_dictionary(uri);
        self
    }

    /// Sets the segmenter user-dictionary URI / path.
    pub fn set_user_dictionary(&mut self, uri: &str) -> &mut Self {
        self.inner.set_segmenter_user_dictionary(uri);
        self
    }

    /// Sets whether whitespace tokens are kept in the output.
    pub fn set_keep_whitespace(&mut self, keep_whitespace: bool) -> &mut Self {
        self.inner.set_segmenter_keep_whitespace(keep_whitespace);
        self
    }

    /// Appends a character filter identified by `kind` with JSON `args`.
    pub fn append_character_filter(&mut self, kind: &str, args: &Value) -> &mut Self {
        self.inner.append_character_filter(kind, args);
        self
    }

    /// Appends a token filter identified by `kind` with JSON `args`.
    pub fn append_token_filter(&mut self, kind: &str, args: &Value) -> &mut Self {
        self.inner.append_token_filter(kind, args);
        self
    }

    /// Builds a [`CoreTokenizer`] from the current configuration.
    pub fn build(&self) -> CoreResult<CoreTokenizer> {
        Ok(CoreTokenizer::from_tokenizer(self.inner.build()?))
    }
}

/// Tokenizer that orchestrates tokenization on behalf of the bindings.
///
/// Internally holds a reusable [`AnalysisWorker`] behind a [`Mutex`], so
/// every call reuses the Viterbi lattice and scratch buffers instead of
/// reallocating them (bindings keep one long-lived `CoreTokenizer`
/// instance, which makes this the natural reuse point). `Mutex` — rather
/// than `RefCell` — keeps `CoreTokenizer: Send + Sync`, which the Python
/// (pyo3) and Node.js (napi) class wrappers require. All binding runtimes
/// call tokenize while effectively single-threaded (GIL / one JS thread /
/// request scope), so the lock is uncontended in practice.
///
/// Returns owned [`TokenView`]s so the bindings never handle borrowed
/// `lindera` tokens directly.
pub struct CoreTokenizer {
    /// The reusable analysis session (lattice + normalization buffer +
    /// scratch), locked per call.
    worker: Mutex<AnalysisWorker>,
}

/// Locks the worker mutex, recovering from poisoning.
///
/// A panic in a previous call may have left the worker's internal buffers
/// in an inconsistent intermediate state, so recovery resets them (the
/// dictionary and filter configuration are unaffected). Capacity built up
/// so far is lost, which is acceptable on this exceptional path.
///
/// # 引数
///
/// * `mutex` - The worker mutex to lock.
///
/// # 戻り値
///
/// A guard for the (possibly reset) worker.
fn lock_worker(mutex: &Mutex<AnalysisWorker>) -> MutexGuard<'_, AnalysisWorker> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            guard.reset();
            guard
        }
    }
}

impl CoreTokenizer {
    /// Builds a tokenizer from segmenter parts, parsing `mode` from a string.
    pub fn from_segmenter(
        mode: &str,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
    ) -> CoreResult<Self> {
        let mode = Mode::from_str(mode)?;
        let segmenter = Segmenter::new(mode, dictionary, user_dictionary);
        Ok(Self::from_tokenizer(Tokenizer::new(segmenter)))
    }

    /// Wraps an already-built lindera [`Tokenizer`], consuming it into the
    /// internal reusable worker (no user-dictionary copy).
    pub fn from_tokenizer(tokenizer: Tokenizer) -> Self {
        Self {
            worker: Mutex::new(tokenizer.into_worker()),
        }
    }

    /// Tokenizes `text`, returning owned [`TokenView`]s.
    ///
    /// Reuses the internal worker's buffers across calls; output is
    /// identical to tokenizing with a fresh lattice.
    pub fn tokenize(&self, text: &str) -> CoreResult<Vec<TokenView>> {
        let mut worker = lock_worker(&self.worker);
        let tokens = worker.tokenize(text)?;
        Ok(tokens.into_iter().map(TokenView::from_token).collect())
    }

    /// Tokenizes `text` and returns the N-best results as `(tokens, cost)` pairs.
    ///
    /// Reuses the internal worker's buffers across calls, like
    /// [`CoreTokenizer::tokenize`].
    pub fn tokenize_nbest(
        &self,
        text: &str,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> CoreResult<Vec<(Vec<TokenView>, i64)>> {
        let mut worker = lock_worker(&self.worker);
        let results = worker.tokenize_nbest(text, n, unique, cost_threshold)?;
        Ok(results
            .into_iter()
            .map(|(tokens, cost)| {
                (
                    tokens.into_iter().map(TokenView::from_token).collect(),
                    cost,
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_new_succeeds() {
        assert!(CoreTokenizerBuilder::new().is_ok());
    }

    #[test]
    fn set_mode_accepts_known_modes() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        assert!(builder.set_mode("normal").is_ok());
        assert!(builder.set_mode("decompose").is_ok());
    }

    #[test]
    fn set_mode_rejects_unknown_mode() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        assert!(builder.set_mode("definitely-not-a-mode").is_err());
    }

    #[test]
    fn infallible_setters_chain() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        builder
            .set_dictionary("embedded://ipadic")
            .set_keep_whitespace(true)
            .append_token_filter("japanese_compound_word", &Value::Object(Default::default()));
        // Reaching here means the borrow-returning setters compose.
    }

    /// The pyo3 (Python) and napi (Node.js) class wrappers require the
    /// wrapped type to be `Send + Sync`; losing either auto-trait would be
    /// a de-facto breaking change for every binding. The `Mutex` (rather
    /// than `RefCell`) around the internal worker exists exactly to
    /// preserve this.
    #[test]
    fn core_tokenizer_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CoreTokenizer>();
        assert_send_sync::<CoreTokenizerBuilder>();
    }

    #[cfg(feature = "embed-ipadic")]
    mod with_ipadic {
        use super::super::*;

        fn ipadic_tokenizer() -> CoreTokenizer {
            let dictionary = match lindera::dictionary::load_dictionary("embedded://ipadic") {
                Ok(dictionary) => dictionary,
                Err(err) => panic!("failed to load embedded IPADIC: {err}"),
            };
            match CoreTokenizer::from_segmenter("normal", dictionary, None) {
                Ok(tokenizer) => tokenizer,
                Err(err) => panic!("failed to build CoreTokenizer: {err}"),
            }
        }

        /// Repeated calls through the internal reused worker must keep
        /// producing identical output (lattice-reuse regression gate).
        #[test]
        fn tokenize_repeated_calls_are_stable() {
            let tokenizer = ipadic_tokenizer();
            let first = match tokenizer.tokenize("すもももももももものうち") {
                Ok(tokens) => tokens,
                Err(err) => panic!("tokenize failed: {err}"),
            };
            assert!(!first.is_empty());
            for _ in 0..100 {
                let again = match tokenizer.tokenize("すもももももももものうち") {
                    Ok(tokens) => tokens,
                    Err(err) => panic!("tokenize failed: {err}"),
                };
                assert_eq!(first.len(), again.len());
                for (a, b) in first.iter().zip(again.iter()) {
                    assert_eq!(a.surface, b.surface);
                    assert_eq!(a.byte_start, b.byte_start);
                    assert_eq!(a.byte_end, b.byte_end);
                    assert_eq!(a.details, b.details);
                }
            }
        }

        /// N-best calls reuse the same worker and must stay stable too.
        #[test]
        fn tokenize_nbest_repeated_calls_are_stable() {
            let tokenizer = ipadic_tokenizer();
            let first = match tokenizer.tokenize_nbest("すもももももももものうち", 3, false, None)
            {
                Ok(results) => results,
                Err(err) => panic!("tokenize_nbest failed: {err}"),
            };
            assert!(!first.is_empty());
            for _ in 0..10 {
                let again =
                    match tokenizer.tokenize_nbest("すもももももももものうち", 3, false, None)
                    {
                        Ok(results) => results,
                        Err(err) => panic!("tokenize_nbest failed: {err}"),
                    };
                assert_eq!(first.len(), again.len());
                for ((a_tokens, a_cost), (b_tokens, b_cost)) in first.iter().zip(again.iter()) {
                    assert_eq!(a_cost, b_cost);
                    assert_eq!(a_tokens.len(), b_tokens.len());
                }
            }
        }

        /// A panic while the worker lock is held must not wedge the
        /// tokenizer: the next call recovers from the poisoned mutex with a
        /// reset worker and produces correct output.
        #[test]
        fn tokenize_recovers_from_poisoned_lock() {
            use std::sync::Arc;

            let tokenizer = Arc::new(ipadic_tokenizer());
            let expected = match tokenizer.tokenize("すもももももももものうち") {
                Ok(tokens) => tokens,
                Err(err) => panic!("tokenize failed: {err}"),
            };

            // Poison the mutex by panicking while holding the guard.
            let poisoner = Arc::clone(&tokenizer);
            let result = std::thread::spawn(move || {
                let _guard = lock_worker(&poisoner.worker);
                panic!("intentional panic to poison the worker lock");
            })
            .join();
            assert!(result.is_err(), "poisoning thread must have panicked");
            assert!(tokenizer.worker.is_poisoned(), "lock must be poisoned");

            // The next call must recover and produce identical output.
            let after = match tokenizer.tokenize("すもももももももものうち") {
                Ok(tokens) => tokens,
                Err(err) => panic!("tokenize after poison failed: {err}"),
            };
            assert_eq!(expected.len(), after.len());
            for (a, b) in expected.iter().zip(after.iter()) {
                assert_eq!(a.surface, b.surface);
            }
        }
    }
}
