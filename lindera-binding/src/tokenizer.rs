//! Shared tokenizer build-flow orchestration for the bindings.
//!
//! Each binding's tokenizer wrapper reimplements the same flow: configure a
//! [`lindera::analysis::tokenizer::TokenizerBuilder`], build a
//! [`lindera::analysis::tokenizer::Tokenizer`], and convert the resulting tokens. This
//! module collects that orchestration into [`CoreTokenizerBuilder`] and
//! [`CoreTokenizer`], leaving each binding to do only its FFI-value conversion
//! (`serde_json::Value` ⇔ the host language's argument type) and a thin wrapper.

use std::path::Path;
use std::str::FromStr;
use std::sync::{Mutex, MutexGuard};

use serde_json::Value;

use lindera::analysis::tokenizer::{Tokenizer, TokenizerBuilder};
use lindera::analysis::worker::AnalysisWorker;
use lindera::dictionary::{Dictionary, UserDictionary};
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use lindera::space_penalty::SpacePenaltyConfig;

use crate::error::{CoreError, CoreResult};
use crate::token::{SurfaceView, TokenView};

/// A left-space penalty setting, parsed from the value the bindings pass
/// (same meaning as `segmenter.space_penalty` in a YAML config).
#[derive(Debug, Clone, PartialEq)]
enum SpacePenaltySetting {
    /// `null`: keep the default, i.e. the rules the dictionary ships, if any.
    Default,
    /// `false`: turn the penalty off.
    Off,
    /// `true`: require the rules the dictionary ships.
    FromDictionary,
    /// An object with `rules`: apply these rules instead.
    Rules(SpacePenaltyConfig),
}

impl SpacePenaltySetting {
    /// Parses a binding-supplied value.
    ///
    /// # Arguments
    ///
    /// * `value` - `null`, a boolean, or an object such as
    ///   `{"rules": [{"pos": ["JKS"], "cost": 6000}]}`.
    ///
    /// # Returns
    ///
    /// The setting, or an invalid-argument error for any other value.
    fn from_value(value: &Value) -> CoreResult<Self> {
        match value {
            Value::Null => Ok(Self::Default),
            Value::Bool(false) => Ok(Self::Off),
            Value::Bool(true) => Ok(Self::FromDictionary),
            Value::Object(_) => serde_json::from_value(value.clone())
                .map(Self::Rules)
                .map_err(|err| {
                    CoreError::invalid_argument(format!("invalid space_penalty rules: {err}"))
                }),
            other => Err(CoreError::invalid_argument(format!(
                "space_penalty must be null, a boolean or an object with \"rules\", got {other}"
            ))),
        }
    }

    /// Applies the setting to a segmenter built with [`Segmenter::new`],
    /// which already carries the dictionary's default rules.
    ///
    /// # Arguments
    ///
    /// * `segmenter` - The segmenter to configure.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or an error when the dictionary ships no rules for
    /// [`Self::FromDictionary`] or the rules cannot be applied to it.
    fn apply(&self, segmenter: &mut Segmenter) -> CoreResult<()> {
        match self {
            Self::Default => {}
            Self::Off => segmenter.set_space_penalty(None)?,
            Self::FromDictionary => segmenter.set_space_penalty_from_dictionary()?,
            Self::Rules(config) => segmenter.set_space_penalty(Some(config.clone()))?,
        }
        Ok(())
    }
}

/// Builds the segmentation mode from the mode name the bindings pass and the
/// penalty they may pass with it (#1215).
///
/// # Arguments
///
/// * `mode` - `"normal"` or `"decompose"`.
/// * `penalty` - The penalty for decompose mode, or `None` for the default.
///
/// # Returns
///
/// The mode, or an invalid-argument error for an unknown mode name or for a
/// penalty with `"normal"`, which has no use for one.
fn mode_with_penalty(mode: &str, penalty: Option<Penalty>) -> CoreResult<Mode> {
    match (Mode::from_str(mode)?, penalty) {
        (Mode::Decompose(_), Some(penalty)) => Ok(Mode::Decompose(penalty)),
        (Mode::Normal, Some(_)) => Err(CoreError::invalid_argument(
            "a penalty applies only to decompose mode, got mode \"normal\"",
        )),
        (mode, None) => Ok(mode),
    }
}

/// Builder that orchestrates tokenizer configuration on behalf of the bindings.
///
/// Wraps [`lindera::analysis::tokenizer::TokenizerBuilder`]; filter arguments are passed
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

    /// Sets the segmenter mode from a string and, for decompose mode, an
    /// optional penalty.
    ///
    /// # Arguments
    ///
    /// * `mode` - `"normal"` or `"decompose"`.
    /// * `penalty` - The penalty for `"decompose"`; `None` keeps the default.
    ///
    /// # Returns
    ///
    /// The builder, or an invalid-argument error for an unknown mode or for a
    /// penalty with `"normal"`.
    pub fn set_mode(&mut self, mode: &str, penalty: Option<Penalty>) -> CoreResult<&mut Self> {
        let mode = mode_with_penalty(mode, penalty)?;
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
    ///
    /// Kept whitespace also stays in the lattice, so the other tokens can
    /// differ from those of the default output, which skips whitespace as
    /// MeCab does.
    ///
    /// # Arguments
    ///
    /// * `keep_whitespace` - `true` to output whitespace tokens.
    ///
    /// # Returns
    ///
    /// The builder, for chaining.
    pub fn set_keep_whitespace(&mut self, keep_whitespace: bool) -> &mut Self {
        self.inner.set_segmenter_keep_whitespace(keep_whitespace);
        self
    }

    /// Sets whether whitespace is skipped in the lattice (see
    /// `Segmenter::skip_whitespace`), with the same meaning as
    /// `segmenter.skip_whitespace` in a YAML config:
    ///
    /// - `Some(true)` skips whitespace, so the words on either side connect
    ///   directly, as MeCab does;
    /// - `Some(false)` keeps whitespace in the lattice as a node of its own,
    ///   as before v7; it is still dropped from the output;
    /// - `None` removes the setting, so the default of the dictionary's
    ///   `metadata.json` applies (skipping, except with SudachiDict).
    ///
    /// A value set here overrides the dictionary's default and a setting
    /// loaded from a configuration file. It has no effect while whitespace
    /// tokens are kept ([`Self::set_keep_whitespace`]), because kept
    /// whitespace always stays in the lattice.
    ///
    /// # Arguments
    ///
    /// * `skip_whitespace` - The setting, or `None` for the dictionary's
    ///   default.
    ///
    /// # Returns
    ///
    /// The builder, for chaining.
    pub fn set_skip_whitespace(&mut self, skip_whitespace: Option<bool>) -> &mut Self {
        match skip_whitespace {
            Some(skip_whitespace) => {
                self.inner.set_segmenter_skip_whitespace(skip_whitespace);
            }
            None => {
                self.inner.reset_segmenter_skip_whitespace();
            }
        }
        self
    }

    /// Sets the left-space penalty (Korean; see `Segmenter::space_penalty`)
    /// from a value with the same meaning as `segmenter.space_penalty` in a
    /// YAML config:
    ///
    /// - `null` restores the default: the rules the dictionary ships, if any
    ///   (ko-dic ships mecab-ko-dic's rules, so they apply unless turned off);
    /// - `false` turns the penalty off;
    /// - `true` requires the dictionary's rules (building fails if it ships
    ///   none);
    /// - an object such as `{"rules": [{"pos": ["JKS"], "cost": 6000}]}`
    ///   applies those rules instead.
    ///
    /// # Arguments
    ///
    /// * `value` - The setting, converted from the binding's native value.
    ///
    /// # Returns
    ///
    /// A mutable reference to `self`, for chaining, or an invalid-argument
    /// error when `value` is not one of the forms above.
    pub fn set_space_penalty(&mut self, value: &Value) -> CoreResult<&mut Self> {
        match SpacePenaltySetting::from_value(value)? {
            SpacePenaltySetting::Default => {
                self.inner.reset_segmenter_space_penalty();
            }
            SpacePenaltySetting::Off => {
                self.inner.set_segmenter_space_penalty(None);
            }
            SpacePenaltySetting::FromDictionary => {
                self.inner.set_segmenter_space_penalty_from_dictionary(true);
            }
            SpacePenaltySetting::Rules(config) => {
                self.inner.set_segmenter_space_penalty(Some(&config));
            }
        }
        Ok(self)
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
    ///
    /// # Returns
    ///
    /// The tokenizer, or an error when no dictionary is configured, a
    /// dictionary cannot be loaded, a setting or filter is invalid, or the
    /// user dictionary has a context ID outside the system dictionary's
    /// connection matrix.
    pub fn build(&self) -> CoreResult<CoreTokenizer> {
        self.build_with_dictionaries(None, None)
    }

    /// Builds a [`CoreTokenizer`] from the current configuration with
    /// already-loaded dictionaries in place of the configured ones. This is
    /// how a binding that takes a dictionary instance (one loaded from bytes,
    /// for example) applies every other setting of the builder to it.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - The system dictionary to use instead of the one
    ///   [`Self::set_dictionary`] set; `None` loads that one.
    /// * `user_dictionary` - The user dictionary to use instead of the one
    ///   [`Self::set_user_dictionary`] set; `None` loads that one, if any.
    ///
    /// # Returns
    ///
    /// The tokenizer, or an error as [`Self::build`] reports it.
    pub fn build_with_dictionaries(
        &self,
        dictionary: Option<Dictionary>,
        user_dictionary: Option<UserDictionary>,
    ) -> CoreResult<CoreTokenizer> {
        Ok(CoreTokenizer::from_tokenizer(
            self.inner
                .build_with_dictionaries(dictionary, user_dictionary)?,
        ))
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
/// # Arguments
///
/// * `mutex` - The worker mutex to lock.
///
/// # Returns
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
    ///
    /// The user dictionary is checked against the dictionary's connection
    /// matrix ([`Segmenter::try_new`]), so a user dictionary written for
    /// another dictionary is an error here rather than a panic at
    /// tokenization.
    ///
    /// # Arguments
    ///
    /// * `mode` - `"normal"` or `"decompose"`.
    /// * `dictionary` - The system dictionary.
    /// * `user_dictionary` - An optional user dictionary.
    ///
    /// # Returns
    ///
    /// The tokenizer, or an error for an invalid mode or for a user
    /// dictionary with a context ID outside the dictionary's connection
    /// matrix.
    pub fn from_segmenter(
        mode: &str,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
    ) -> CoreResult<Self> {
        let mode = Mode::from_str(mode)?;
        let segmenter = Segmenter::try_new(mode, dictionary, user_dictionary)?;
        Ok(Self::from_tokenizer(Tokenizer::new(segmenter)))
    }

    /// Like [`Self::from_segmenter`], with a left-space penalty setting
    /// applied to the segmenter. The bindings' `Tokenizer` constructors,
    /// which take a dictionary instance and no builder, use this to accept
    /// the setting [`CoreTokenizerBuilder::set_space_penalty`] takes.
    ///
    /// # Arguments
    ///
    /// * `mode` - `"normal"` or `"decompose"`.
    /// * `penalty` - The penalty for `"decompose"`, as
    ///   [`CoreTokenizerBuilder::set_mode`] takes it; `None` keeps the
    ///   default.
    /// * `dictionary` - The system dictionary.
    /// * `user_dictionary` - An optional user dictionary, checked against
    ///   the dictionary's connection matrix as in [`Self::from_segmenter`].
    /// * `space_penalty` - The setting, in the forms
    ///   [`CoreTokenizerBuilder::set_space_penalty`] accepts (`null` keeps the
    ///   dictionary's default rules).
    ///
    /// # Returns
    ///
    /// The tokenizer, or an error for an invalid mode or setting, for a
    /// penalty with `"normal"`, for `true` with a dictionary that ships no
    /// rules, for rules that cannot be applied to the dictionary, or for a
    /// user dictionary with a context ID outside the dictionary's connection
    /// matrix.
    pub fn from_segmenter_with_space_penalty(
        mode: &str,
        penalty: Option<Penalty>,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
        space_penalty: &Value,
    ) -> CoreResult<Self> {
        let setting = SpacePenaltySetting::from_value(space_penalty)?;
        let mode = mode_with_penalty(mode, penalty)?;
        let mut segmenter = Segmenter::try_new(mode, dictionary, user_dictionary)?;
        setting.apply(&mut segmenter)?;
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

    /// Tokenizes `text`, returning surface-only views ([`SurfaceView`])
    /// without materializing any morphological details.
    ///
    /// This is the fast path for wakati-style callers: it skips the
    /// per-token details loading and string materialization that
    /// [`CoreTokenizer::tokenize`] pays (~14 allocations per IPADIC token
    /// down to ~1), while reusing the internal worker's buffers like the
    /// other methods. Token filters configured on the tokenizer still run
    /// (some may load details internally); the saving on this path is the
    /// FFI-side materialization.
    pub fn tokenize_surfaces(&self, text: &str) -> CoreResult<Vec<SurfaceView>> {
        let mut worker = lock_worker(&self.worker);
        let tokens = worker.tokenize(text)?;
        Ok(tokens.into_iter().map(SurfaceView::from_token).collect())
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
        assert!(builder.set_mode("normal", None).is_ok());
        assert!(builder.set_mode("decompose", None).is_ok());
    }

    #[test]
    fn set_mode_rejects_unknown_mode() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        assert!(builder.set_mode("definitely-not-a-mode", None).is_err());
    }

    /// A penalty goes with decompose mode, and is an invalid argument with
    /// normal mode, which has no use for one (#1215).
    #[test]
    fn mode_with_penalty_combines_the_mode_and_the_penalty() {
        let penalty = Penalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: 4000,
            other_penalty_length_threshold: 8,
            other_penalty_length_penalty: 0,
        };
        assert_eq!(mode_with_penalty("normal", None).ok(), Some(Mode::Normal));
        assert_eq!(
            mode_with_penalty("decompose", None).ok(),
            Some(Mode::Decompose(Penalty::default()))
        );
        assert_eq!(
            mode_with_penalty("decompose", Some(penalty.clone())).ok(),
            Some(Mode::Decompose(penalty.clone()))
        );
        match mode_with_penalty("normal", Some(penalty.clone())) {
            Ok(mode) => panic!("a penalty with normal mode gave {mode:?}"),
            Err(err) => assert_eq!(err.kind(), crate::error::ErrorKind::InvalidArgument),
        }
        assert!(mode_with_penalty("definitely-not-a-mode", Some(penalty)).is_err());

        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        assert!(
            builder
                .set_mode("normal", Some(Penalty::default()))
                .is_err()
        );
    }

    #[test]
    fn infallible_setters_chain() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        builder
            .set_dictionary("embedded://ipadic")
            .set_keep_whitespace(true)
            .set_skip_whitespace(Some(false))
            .append_token_filter("japanese_compound_word", &Value::Object(Default::default()));
        // Reaching here means the borrow-returning setters compose.
    }

    #[test]
    fn set_space_penalty_accepts_the_config_forms() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        for value in [
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            serde_json::json!({ "rules": [{ "pos": ["JKS"], "cost": 6000 }] }),
            serde_json::json!({ "rules": [] }),
        ] {
            assert!(
                builder.set_space_penalty(&value).is_ok(),
                "rejected {value}"
            );
        }
    }

    #[test]
    fn set_space_penalty_rejects_other_values() {
        let mut builder = CoreTokenizerBuilder::new().expect("builder");
        for value in [
            serde_json::json!(1),
            serde_json::json!("false"),
            serde_json::json!(["JKS"]),
            serde_json::json!({ "rules": "JKS" }),
            serde_json::json!({ "rules": [{ "pos": ["JKS"] }] }),
        ] {
            let err = match builder.set_space_penalty(&value) {
                Ok(_) => panic!("accepted {value}"),
                Err(err) => err,
            };
            assert_eq!(err.kind(), crate::error::ErrorKind::InvalidArgument);
        }
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

        /// An empty input gets one N-best result without tokens (#1147),
        /// which every binding's N-best method returns.
        #[test]
        fn tokenize_nbest_empty_input_gives_one_result() {
            let tokenizer = ipadic_tokenizer();
            let results = match tokenizer.tokenize_nbest("", 3, false, None) {
                Ok(results) => results,
                Err(err) => panic!("tokenize_nbest failed: {err}"),
            };
            assert_eq!(results.len(), 1);
            assert!(results[0].0.is_empty());
        }

        /// The surface-only fast path must yield exactly the surfaces and
        /// byte offsets of the full tokenize path, for repeated calls.
        #[test]
        fn tokenize_surfaces_matches_tokenize() {
            let tokenizer = ipadic_tokenizer();
            let texts = [
                "すもももももももものうち",
                "関西国際空港限定トートバッグ",
                "",
            ];
            for _ in 0..3 {
                for text in texts {
                    let full = match tokenizer.tokenize(text) {
                        Ok(tokens) => tokens,
                        Err(err) => panic!("tokenize failed: {err}"),
                    };
                    let surfaces = match tokenizer.tokenize_surfaces(text) {
                        Ok(views) => views,
                        Err(err) => panic!("tokenize_surfaces failed: {err}"),
                    };
                    assert_eq!(full.len(), surfaces.len(), "count for {text:?}");
                    for (a, b) in full.iter().zip(surfaces.iter()) {
                        assert_eq!(a.surface, b.surface);
                        assert_eq!(a.byte_start, b.byte_start);
                        assert_eq!(a.byte_end, b.byte_end);
                    }
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

        /// IPADIC ships no left-space penalty rules: requiring them (`true`)
        /// makes the build fail, while the default and `false` build fine.
        #[test]
        fn space_penalty_true_fails_on_a_dictionary_without_rules() {
            for (value, should_build) in [
                (Value::Null, true),
                (Value::Bool(false), true),
                (Value::Bool(true), false),
            ] {
                let mut builder = CoreTokenizerBuilder::new().expect("builder");
                builder
                    .set_dictionary("embedded://ipadic")
                    .set_space_penalty(&value)
                    .expect("valid value");
                assert_eq!(builder.build().is_ok(), should_build, "value {value}");
            }
        }

        /// The constructor path (a dictionary instance) behaves like the
        /// builder path.
        #[test]
        fn from_segmenter_with_space_penalty_on_ipadic() {
            for (value, should_build) in [
                (Value::Null, true),
                (Value::Bool(false), true),
                (Value::Bool(true), false),
                (
                    serde_json::json!({ "rules": [{ "pos": ["助詞"], "cost": 1000 }] }),
                    true,
                ),
            ] {
                let dictionary = match lindera::dictionary::load_dictionary("embedded://ipadic") {
                    Ok(dictionary) => dictionary,
                    Err(err) => panic!("failed to load embedded IPADIC: {err}"),
                };
                let result = CoreTokenizer::from_segmenter_with_space_penalty(
                    "normal", None, dictionary, None, &value,
                );
                assert_eq!(result.is_ok(), should_build, "value {value}");
            }
        }

        /// Loads the embedded IPADIC.
        fn ipadic() -> Dictionary {
            match lindera::dictionary::load_dictionary("embedded://ipadic") {
                Ok(dictionary) => dictionary,
                Err(err) => panic!("failed to load embedded IPADIC: {err}"),
            }
        }

        /// Builds `builder` with the given dictionaries and returns the
        /// surfaces of `text`.
        fn surfaces(
            builder: &CoreTokenizerBuilder,
            dictionary: Option<Dictionary>,
            user_dictionary: Option<UserDictionary>,
            text: &str,
        ) -> Vec<String> {
            let tokenizer = match builder.build_with_dictionaries(dictionary, user_dictionary) {
                Ok(tokenizer) => tokenizer,
                Err(err) => panic!("build failed: {err}"),
            };
            match tokenizer.tokenize_surfaces(text) {
                Ok(views) => views.into_iter().map(|view| view.surface).collect(),
                Err(err) => panic!("tokenize_surfaces failed: {err}"),
            }
        }

        /// A penalty passed with decompose mode reaches the segmenter, through
        /// the builder and through the constructor path (#1215): without
        /// length penalties, `関西国際空港` stays whole.
        #[test]
        fn decompose_penalty_reaches_the_segmenter() {
            let text = "関西国際空港限定トートバッグ";
            let no_length_penalty = Penalty {
                kanji_penalty_length_penalty: 0,
                other_penalty_length_penalty: 0,
                ..Penalty::default()
            };
            for (penalty, expected) in [
                (None, vec!["関西", "国際", "空港", "限定", "トートバッグ"]),
                (
                    Some(no_length_penalty),
                    vec!["関西国際空港", "限定", "トートバッグ"],
                ),
            ] {
                let mut builder = CoreTokenizerBuilder::new().expect("builder");
                builder
                    .set_dictionary("embedded://ipadic")
                    .set_mode("decompose", penalty.clone())
                    .expect("valid mode");
                assert_eq!(surfaces(&builder, None, None, text), expected, "builder");

                let tokenizer = match CoreTokenizer::from_segmenter_with_space_penalty(
                    "decompose",
                    penalty,
                    ipadic(),
                    None,
                    &Value::Null,
                ) {
                    Ok(tokenizer) => tokenizer,
                    Err(err) => panic!("build failed: {err}"),
                };
                let from_constructor: Vec<String> = match tokenizer.tokenize_surfaces(text) {
                    Ok(views) => views.into_iter().map(|view| view.surface).collect(),
                    Err(err) => panic!("tokenize_surfaces failed: {err}"),
                };
                assert_eq!(from_constructor, expected, "constructor");
            }
        }

        /// A dictionary instance takes every other setting of the builder,
        /// the filters included, as a dictionary set by URI does (#1138).
        #[test]
        fn build_with_dictionaries_applies_the_builder_settings() {
            let mut builder = CoreTokenizerBuilder::new().expect("builder");
            builder
                .set_mode("normal", None)
                .expect("valid mode")
                .set_keep_whitespace(true)
                .append_character_filter(
                    "unicode_normalize",
                    &serde_json::json!({ "kind": "nfkc" }),
                )
                .append_token_filter("lowercase", &serde_json::json!({}));
            let text = "Ｌｉｎｄｅｒａ 東京";
            let from_instance = surfaces(&builder, Some(ipadic()), None, text);

            builder.set_dictionary("embedded://ipadic");
            let by_uri = surfaces(&builder, None, None, text);
            assert_eq!(by_uri, ["lindera", " ", "東京"]);
            assert_eq!(from_instance, by_uri);
        }

        /// A user dictionary instance applies to a dictionary set by URI
        /// (#1138).
        #[test]
        fn build_with_dictionaries_adds_a_user_dictionary_to_the_uri_dictionary() {
            let mut builder = CoreTokenizerBuilder::new().expect("builder");
            builder.set_dictionary("embedded://ipadic");
            let baseline = surfaces(&builder, None, None, "東京スカイツリー");
            assert!(baseline.len() > 1, "fixture assumption: {baseline:?}");

            let csv = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources/user_dict/ipadic_simple_userdic.csv");
            let user_dictionary = match lindera::dictionary::load_user_dictionary(
                csv.to_str().expect("UTF-8 path"),
                &ipadic().metadata,
            ) {
                Ok(user_dictionary) => user_dictionary,
                Err(err) => panic!("failed to load the user dictionary: {err}"),
            };
            assert_eq!(
                surfaces(&builder, None, Some(user_dictionary), "東京スカイツリー"),
                ["東京スカイツリー"]
            );
        }

        /// Builds a user dictionary whose entry has context IDs 60000, far
        /// outside IPADIC's 1316 x 1316 connection matrix.
        fn out_of_range_user_dictionary() -> UserDictionary {
            let csv = "東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,\
                       東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー\n";
            match lindera::dictionary::DictionaryBuilder::new((*ipadic().metadata).clone())
                .build_user_dict_from_reader(csv.as_bytes())
            {
                Ok(user_dictionary) => user_dictionary,
                Err(err) => panic!("failed to build the user dictionary: {err}"),
            }
        }

        /// The `Tokenizer` constructors of the bindings and their builders
        /// reject a user dictionary whose context IDs lie outside the
        /// dictionary's connection matrix, instead of building a tokenizer
        /// that panics when the entry matches the input (#1171).
        #[test]
        fn user_dictionary_outside_the_matrix_is_an_error() {
            let results = [
                CoreTokenizer::from_segmenter(
                    "normal",
                    ipadic(),
                    Some(out_of_range_user_dictionary()),
                )
                .err(),
                CoreTokenizer::from_segmenter_with_space_penalty(
                    "normal",
                    None,
                    ipadic(),
                    Some(out_of_range_user_dictionary()),
                    &Value::Null,
                )
                .err(),
                CoreTokenizerBuilder::new()
                    .expect("builder")
                    .build_with_dictionaries(Some(ipadic()), Some(out_of_range_user_dictionary()))
                    .err(),
            ];
            for (case, err) in results.into_iter().enumerate() {
                match err {
                    Some(err) => {
                        assert_eq!(
                            err.kind(),
                            crate::error::ErrorKind::Parse,
                            "case {case}: {err}"
                        );
                        assert!(
                            err.to_string()
                                .contains("left context id 60000 is out of range"),
                            "case {case}: {err}"
                        );
                    }
                    None => panic!("case {case}: built a tokenizer"),
                }
            }
        }

        /// A `.csv` user dictionary set on the builder by path is built for
        /// the builder's dictionary, so a row outside its connection matrix
        /// is reported by its CSV line and surface (#1187).
        #[test]
        fn builder_names_the_csv_line_of_a_user_dictionary_outside_the_matrix() {
            let dir = tempfile::tempdir().expect("temporary directory");
            let csv = dir.path().join("userdic.csv");
            if let Err(err) = std::fs::write(
                &csv,
                "東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,\
                 東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー\n",
            ) {
                panic!("failed to write the user dictionary: {err}");
            }

            let mut builder = CoreTokenizerBuilder::new().expect("builder");
            builder
                .set_dictionary("embedded://ipadic")
                .set_user_dictionary(csv.to_str().expect("UTF-8 path"));
            match builder.build() {
                Ok(_) => panic!("built a tokenizer"),
                Err(err) => assert!(
                    err.to_string().contains(
                        "user dictionary line 1 (surface: '東京スカイツリー'): left context id \
                         60000 is out of range"
                    ),
                    "{err}"
                ),
            }
        }

        /// A dictionary instance given to the builder takes the space-penalty
        /// setting as the constructor path does.
        #[test]
        fn build_with_dictionaries_space_penalty_on_ipadic() {
            for (value, should_build) in [
                (Value::Null, true),
                (Value::Bool(false), true),
                (Value::Bool(true), false),
                (
                    serde_json::json!({ "rules": [{ "pos": ["助詞"], "cost": 1000 }] }),
                    true,
                ),
            ] {
                let mut builder = CoreTokenizerBuilder::new().expect("builder");
                builder.set_space_penalty(&value).expect("valid value");
                let result = builder.build_with_dictionaries(Some(ipadic()), None);
                assert_eq!(result.is_ok(), should_build, "value {value}");
            }
        }

        /// Text whose reading depends on whitespace skipping: with whitespace
        /// skipped IPADIC reads `が` as a particle (助詞), as MeCab does; with
        /// the whitespace node in the lattice, as a conjunction (接続詞).
        const SPACED_TEXT: &str = "Google が 新しい";

        /// Builds `builder` with the given dictionary instance (`None` loads
        /// the one set by URI) and returns each token of [`SPACED_TEXT`] as
        /// (surface, first part-of-speech field).
        fn readings(
            builder: &CoreTokenizerBuilder,
            dictionary: Option<Dictionary>,
        ) -> Vec<(String, String)> {
            let tokenizer = match builder.build_with_dictionaries(dictionary, None) {
                Ok(tokenizer) => tokenizer,
                Err(err) => panic!("build failed: {err}"),
            };
            match tokenizer.tokenize(SPACED_TEXT) {
                Ok(tokens) => tokens
                    .into_iter()
                    .map(|t| (t.surface, t.details.first().cloned().unwrap_or_default()))
                    .collect(),
                Err(err) => panic!("tokenize failed: {err}"),
            }
        }

        /// The first part-of-speech field of `が` in [`SPACED_TEXT`].
        fn reading_of_ga(builder: &CoreTokenizerBuilder, dictionary: Option<Dictionary>) -> String {
            let readings = readings(builder, dictionary);
            match readings.iter().find(|(surface, _)| surface == "が") {
                Some((_, pos)) => pos.clone(),
                None => panic!("no token with surface が: {readings:?}"),
            }
        }

        /// `set_skip_whitespace` turns whitespace skipping off and on, the
        /// last call wins, and `None` goes back to the dictionary's default
        /// (skipping, for IPADIC).
        #[test]
        fn set_skip_whitespace_changes_the_ipadic_reading() {
            let cases: [(&[Option<bool>], &str); 5] = [
                (&[], "助詞"),
                (&[Some(true)], "助詞"),
                (&[Some(false)], "接続詞"),
                (&[Some(false), None], "助詞"),
                (&[Some(true), Some(false)], "接続詞"),
            ];
            for (settings, expected) in cases {
                let mut builder = CoreTokenizerBuilder::new().expect("builder");
                builder.set_dictionary("embedded://ipadic");
                for setting in settings {
                    builder.set_skip_whitespace(*setting);
                }
                assert_eq!(
                    reading_of_ga(&builder, None),
                    expected,
                    "settings {settings:?}"
                );
            }
        }

        /// A value set with `set_skip_whitespace` overrides the default of
        /// the dictionary's metadata, and `None` restores that default: an
        /// IPADIC whose metadata turns skipping off, as SudachiDict's does,
        /// reads `が` as a conjunction unless skipping is turned on.
        #[test]
        fn set_skip_whitespace_overrides_the_dictionary_default() {
            let no_skip_ipadic = || {
                let mut dictionary = ipadic();
                std::sync::Arc::make_mut(&mut dictionary.metadata).skip_whitespace = Some(false);
                dictionary
            };
            let mut builder = CoreTokenizerBuilder::new().expect("builder");
            assert_eq!(reading_of_ga(&builder, Some(no_skip_ipadic())), "接続詞");
            builder.set_skip_whitespace(Some(true));
            assert_eq!(reading_of_ga(&builder, Some(no_skip_ipadic())), "助詞");
            builder.set_skip_whitespace(None);
            assert_eq!(reading_of_ga(&builder, Some(no_skip_ipadic())), "接続詞");
        }

        /// A dictionary instance takes the setting as a dictionary set by URI
        /// does; lindera-wasm's `setDictionaryInstance()` builds this way.
        #[test]
        fn set_skip_whitespace_applies_to_a_dictionary_instance() {
            let mut builder = CoreTokenizerBuilder::new().expect("builder");
            assert_eq!(reading_of_ga(&builder, Some(ipadic())), "助詞");
            builder.set_skip_whitespace(Some(false));
            assert_eq!(reading_of_ga(&builder, Some(ipadic())), "接続詞");
        }

        /// Kept whitespace always stays in the lattice, so skipping cannot
        /// take effect: the output, whitespace tokens included, is the same
        /// with and without `set_skip_whitespace(Some(true))`.
        #[test]
        fn set_skip_whitespace_has_no_effect_with_kept_whitespace() {
            let mut builder = CoreTokenizerBuilder::new().expect("builder");
            builder
                .set_dictionary("embedded://ipadic")
                .set_keep_whitespace(true);
            let kept = readings(&builder, None);
            assert!(
                kept.iter().any(|(surface, _)| surface == " "),
                "fixture assumption: whitespace tokens are kept: {kept:?}"
            );
            builder.set_skip_whitespace(Some(true));
            assert_eq!(readings(&builder, None), kept);
        }
    }

    #[cfg(feature = "embed-ko-dic")]
    mod with_ko_dic {
        use super::super::*;

        /// First part-of-speech tag of each token for `text`, built with the
        /// given space-penalty setting.
        fn tags(space_penalty: &Value, text: &str) -> Vec<(String, String)> {
            let mut builder = CoreTokenizerBuilder::new().expect("builder");
            builder
                .set_dictionary("embedded://ko-dic")
                .set_space_penalty(space_penalty)
                .expect("valid value");
            let tokenizer = match builder.build() {
                Ok(tokenizer) => tokenizer,
                Err(err) => panic!("build failed: {err}"),
            };
            match tokenizer.tokenize(text) {
                Ok(tokens) => tokens
                    .into_iter()
                    .map(|t| (t.surface, t.details.first().cloned().unwrap_or_default()))
                    .collect(),
                Err(err) => panic!("tokenize failed: {err}"),
            }
        }

        /// ko-dic ships mecab-ko-dic's rules, so the penalty applies by
        /// default; `false` turns it off, `true` and the shipped rules given
        /// explicitly behave like the default (#1052).
        #[test]
        fn space_penalty_setting_changes_ko_dic_output() {
            let text = "검색 이 잘 된다";
            let default = tags(&Value::Null, text);
            let off = tags(&Value::Bool(false), text);
            let required = tags(&Value::Bool(true), text);
            let shipped = serde_json::json!({ "rules": [
                { "pos": ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], "cost": 3000 },
                { "pos": ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], "cost": 6000 }
            ] });
            let explicit = tags(&shipped, text);

            assert_ne!(
                default, off,
                "turning the penalty off must change the output"
            );
            assert_eq!(default, required);
            assert_eq!(default, explicit);
            // With the penalty, `이` after a space is no longer the subject
            // particle JKS (mecab-ko reads it as the interjection IC).
            assert!(
                off.iter().any(|(s, tag)| s == "이" && tag == "JKS"),
                "{off:?}"
            );
            assert!(
                !default.iter().any(|(s, tag)| s == "이" && tag == "JKS"),
                "{default:?}"
            );
        }

        /// Building from a dictionary instance with the constructor path
        /// honours the same setting.
        #[test]
        fn from_segmenter_with_space_penalty_matches_the_builder() {
            let text = "검색 이 잘 된다";
            for value in [Value::Null, Value::Bool(false), Value::Bool(true)] {
                let dictionary = match lindera::dictionary::load_dictionary("embedded://ko-dic") {
                    Ok(dictionary) => dictionary,
                    Err(err) => panic!("failed to load embedded ko-dic: {err}"),
                };
                let tokenizer = match CoreTokenizer::from_segmenter_with_space_penalty(
                    "normal", None, dictionary, None, &value,
                ) {
                    Ok(tokenizer) => tokenizer,
                    Err(err) => panic!("build failed for {value}: {err}"),
                };
                let from_instance: Vec<(String, String)> = match tokenizer.tokenize(text) {
                    Ok(tokens) => tokens
                        .into_iter()
                        .map(|t| (t.surface, t.details.first().cloned().unwrap_or_default()))
                        .collect(),
                    Err(err) => panic!("tokenize failed: {err}"),
                };
                assert_eq!(from_instance, tags(&value, text), "value {value}");
            }
        }

        /// A dictionary instance given to the builder (as lindera-wasm does
        /// after `loadDictionaryFromBytes`) honours the same setting.
        #[test]
        fn build_with_dictionaries_matches_the_builder() {
            let text = "검색 이 잘 된다";
            for value in [Value::Null, Value::Bool(false), Value::Bool(true)] {
                let dictionary = match lindera::dictionary::load_dictionary("embedded://ko-dic") {
                    Ok(dictionary) => dictionary,
                    Err(err) => panic!("failed to load embedded ko-dic: {err}"),
                };
                let mut builder = CoreTokenizerBuilder::new().expect("builder");
                builder.set_space_penalty(&value).expect("valid value");
                let tokenizer = match builder.build_with_dictionaries(Some(dictionary), None) {
                    Ok(tokenizer) => tokenizer,
                    Err(err) => panic!("build failed for {value}: {err}"),
                };
                let from_instance: Vec<(String, String)> = match tokenizer.tokenize(text) {
                    Ok(tokens) => tokens
                        .into_iter()
                        .map(|t| (t.surface, t.details.first().cloned().unwrap_or_default()))
                        .collect(),
                    Err(err) => panic!("tokenize failed: {err}"),
                };
                assert_eq!(from_instance, tags(&value, text), "value {value}");
            }
        }

        /// ko-dic skips whitespace by default, so `전` in `2년 전 대회` is the
        /// noun NNG, as in mecab-ko; with the whitespace node in the lattice,
        /// which resets the context at the space, it is `저/NP + ㄴ/JX`.
        #[test]
        fn set_skip_whitespace_changes_ko_dic_output() {
            let tag_of_jeon = |skip_whitespace: Option<bool>| {
                let mut builder = CoreTokenizerBuilder::new().expect("builder");
                builder
                    .set_dictionary("embedded://ko-dic")
                    .set_skip_whitespace(skip_whitespace);
                let tokenizer = match builder.build() {
                    Ok(tokenizer) => tokenizer,
                    Err(err) => panic!("build failed: {err}"),
                };
                let tokens = match tokenizer.tokenize("2년 전 대회") {
                    Ok(tokens) => tokens,
                    Err(err) => panic!("tokenize failed: {err}"),
                };
                match tokens.into_iter().find(|t| t.surface == "전") {
                    Some(token) => token.details.first().cloned().unwrap_or_default(),
                    None => panic!("no token with surface 전"),
                }
            };
            assert_eq!(tag_of_jeon(None), "NNG");
            assert_eq!(tag_of_jeon(Some(true)), "NNG");
            assert_eq!(tag_of_jeon(Some(false)), "NP+JX");
        }
    }
}
