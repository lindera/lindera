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
use lindera::space_penalty::SpacePenaltyConfig;
use lindera_analysis::tokenizer::{Tokenizer, TokenizerBuilder};
use lindera_analysis::worker::AnalysisWorker;

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

    /// Like [`Self::from_segmenter`], with a left-space penalty setting
    /// applied to the segmenter. This is how a binding that builds from a
    /// dictionary instance (rather than a URI through
    /// [`CoreTokenizerBuilder`]) honours
    /// [`CoreTokenizerBuilder::set_space_penalty`].
    ///
    /// # Arguments
    ///
    /// * `mode` - `"normal"` or `"decompose"`.
    /// * `dictionary` - The system dictionary.
    /// * `user_dictionary` - An optional user dictionary.
    /// * `space_penalty` - The setting, in the forms
    ///   [`CoreTokenizerBuilder::set_space_penalty`] accepts (`null` keeps the
    ///   dictionary's default rules).
    ///
    /// # Returns
    ///
    /// The tokenizer, or an error for an invalid mode or setting, for `true`
    /// with a dictionary that ships no rules, or for rules that cannot be
    /// applied to the dictionary.
    pub fn from_segmenter_with_space_penalty(
        mode: &str,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
        space_penalty: &Value,
    ) -> CoreResult<Self> {
        let setting = SpacePenaltySetting::from_value(space_penalty)?;
        let mode = Mode::from_str(mode)?;
        let mut segmenter = Segmenter::new(mode, dictionary, user_dictionary);
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

        /// The dictionary-instance path behaves like the builder path.
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
                    "normal", dictionary, None, &value,
                );
                assert_eq!(result.is_ok(), should_build, "value {value}");
            }
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
            let text = "서울 시 에서 출발";
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
            // With the penalty, `시` after a space is no longer the
            // pre-final ending EP.
            assert!(
                off.iter().any(|(s, tag)| s == "시" && tag == "EP"),
                "{off:?}"
            );
            assert!(
                !default.iter().any(|(s, tag)| s == "시" && tag == "EP"),
                "{default:?}"
            );
        }

        /// Building from a dictionary instance (as lindera-wasm does after
        /// `loadDictionaryFromBytes`) honours the same setting.
        #[test]
        fn from_segmenter_with_space_penalty_matches_the_builder() {
            let text = "서울 시 에서 출발";
            for value in [Value::Null, Value::Bool(false), Value::Bool(true)] {
                let dictionary = match lindera::dictionary::load_dictionary("embedded://ko-dic") {
                    Ok(dictionary) => dictionary,
                    Err(err) => panic!("failed to load embedded ko-dic: {err}"),
                };
                let tokenizer = match CoreTokenizer::from_segmenter_with_space_penalty(
                    "normal", dictionary, None, &value,
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
    }
}
