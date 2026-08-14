use std::borrow::Cow;

use lindera_dictionary::mode::Mode;
use lindera_dictionary::viterbi::{Lattice, WordId};

use crate::LinderaResult;
use crate::segmenter::{MAX_SENTENCE_BYTES, Segmenter};
use crate::token::Token;

/// Number of `segment`/`segment_nbest` calls between two automatic shrink
/// checks. A window (rather than a per-call check) amortizes the check and
/// avoids shrink/regrow thrashing on alternating long/short inputs.
const SHRINK_WINDOW_CALLS: u32 = 64;

/// Hysteresis factor for the automatic shrink: the lattice is only shrunk
/// when its capacity exceeds the window's observed need by more than this
/// factor, so a workload hovering around one size never thrashes.
const SHRINK_HYSTERESIS: usize = 4;

/// Floor (in bytes) below which the lattice is never shrunk. Typical
/// line-oriented input stays under this, so the automatic policy never
/// fires for it; worst-case retention above the floor is bounded to a few
/// MB instead of the ~20 MB a 32 KiB sentence can pin.
const SHRINK_FLOOR_BYTES: usize = 4 * 1024;

/// A reusable segmentation session that owns the Viterbi [`Lattice`] and
/// the backtrace scratch buffer, so repeated calls avoid the per-call
/// allocations that `Segmenter::segment` pays.
///
/// A worker is created from a [`Segmenter`] via [`Segmenter::new_worker`]
/// or [`Segmenter::into_worker`] — never constructed directly — so the
/// lattice is permanently tied to one dictionary. Reusing a lattice across
/// dictionaries was the source of a past correctness bug (see
/// `lindera/tests/lattice_reuse_across_dictionaries.rs`); this type makes
/// that misuse unrepresentable.
///
/// Tokens returned by [`SegmentWorker::segment`] borrow the worker, so the
/// borrow checker requires consuming (or cloning from) them before the next
/// call — the usual per-line loop works without annotations:
///
/// ```ignore
/// let mut worker = segmenter.new_worker();
/// for line in lines {
///     let tokens = worker.segment(line)?;
///     for token in &tokens {
///         // use token.surface, ...
///     }
/// } // tokens dropped here; the next iteration may call segment again
/// ```
///
/// The worker automatically bounds retained memory: one 32 KiB sentence
/// grows the lattice to ~20 MB, and without intervention that stays pinned
/// for the worker's lifetime. Every [`SHRINK_WINDOW_CALLS`] calls the
/// worker compares the lattice capacity against the window's largest input
/// (with a [`SHRINK_HYSTERESIS`]x margin and a [`SHRINK_FLOOR_BYTES`]
/// floor) and shrinks it when oversized. [`SegmentWorker::shrink_to`]
/// forces a shrink immediately.
///
/// The worker is `Send + Sync` (all fields are); the `&mut self` API means
/// sharing one worker across threads requires external synchronization —
/// the intended pattern is one worker per thread over a shared `Segmenter`.
pub struct SegmentWorker {
    /// The owned segmenter this worker is permanently bound to.
    segmenter: Segmenter,
    /// The reused Viterbi lattice (grows monotonically between shrinks).
    lattice: Lattice,
    /// Backtrace scratch reused across calls (cleared per sentence by
    /// `tokens_offset_into`).
    offsets: Vec<(usize, WordId)>,
    /// Largest per-call input need (bytes, capped at one sentence) observed
    /// in the current shrink window.
    window_max_needed: usize,
    /// Calls seen in the current shrink window.
    calls_in_window: u32,
}

impl Segmenter {
    /// Creates a reusable [`SegmentWorker`] bound to a clone of this
    /// segmenter.
    ///
    /// The clone is cheap for the dictionary (`Arc`-backed) but deep-copies
    /// the user dictionary if one is configured; when the segmenter itself
    /// is no longer needed, prefer [`Segmenter::into_worker`] to avoid that
    /// copy.
    ///
    /// # 戻り値
    ///
    /// A fresh worker with empty internal buffers.
    pub fn new_worker(&self) -> SegmentWorker {
        SegmentWorker::new(self.clone())
    }

    /// Consumes this segmenter and creates a reusable [`SegmentWorker`]
    /// from it, without cloning the user dictionary.
    ///
    /// # 戻り値
    ///
    /// A fresh worker with empty internal buffers.
    pub fn into_worker(self) -> SegmentWorker {
        SegmentWorker::new(self)
    }
}

impl SegmentWorker {
    /// Creates a worker around an owned segmenter. Private: the only public
    /// entry points are `Segmenter::new_worker`/`into_worker`, which keep
    /// the dictionary↔lattice pairing fixed.
    ///
    /// # 引数
    ///
    /// * `segmenter` - The segmenter this worker is bound to.
    ///
    /// # 戻り値
    ///
    /// A worker with empty internal buffers.
    fn new(segmenter: Segmenter) -> Self {
        Self {
            segmenter,
            lattice: Lattice::default(),
            offsets: Vec::new(),
            window_max_needed: 0,
            calls_in_window: 0,
        }
    }

    /// Segments `text` reusing this worker's internal buffers.
    ///
    /// Produces exactly the same tokens as [`Segmenter::segment`] for the
    /// same input and configuration. The returned tokens borrow both `text`
    /// and this worker, so they must be consumed before the next call.
    ///
    /// # 引数
    ///
    /// * `text` - The input text to segment.
    ///
    /// # 戻り値
    ///
    /// The tokens segmented from `text`, in reading order.
    pub fn segment<'w>(&'w mut self, text: &'w str) -> LinderaResult<Vec<Token<'w>>> {
        self.maybe_auto_shrink(text.len());
        let Self {
            segmenter,
            lattice,
            offsets,
            ..
        } = self;
        segmenter.segment_with_buffers(Cow::Borrowed(text), lattice, offsets)
    }

    /// Segments `text` and returns the top-N results with costs, reusing
    /// this worker's lattice.
    ///
    /// Produces exactly the same results as [`Segmenter::segment_nbest`]
    /// for the same input and configuration.
    ///
    /// # 引数
    ///
    /// * `text` - The input text to segment.
    /// * `n` - Maximum number of segmentations to return.
    /// * `unique` - Deduplicate results with identical word boundaries.
    /// * `cost_threshold` - Discard paths costing more than best + threshold.
    ///
    /// # 戻り値
    ///
    /// Up to `n` `(tokens, cost)` pairs ordered by cost (best first).
    pub fn segment_nbest<'w>(
        &'w mut self,
        text: &'w str,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> LinderaResult<Vec<(Vec<Token<'w>>, i64)>> {
        self.maybe_auto_shrink(text.len());
        let Self {
            segmenter, lattice, ..
        } = self;
        segmenter.segment_nbest_with_lattice(
            Cow::Borrowed(text),
            lattice,
            n,
            unique,
            cost_threshold,
        )
    }

    /// Sets the segmentation mode for subsequent calls.
    ///
    /// The mode carries no lattice state (it is passed anew to every
    /// sentence), so switching per call is safe.
    ///
    /// # 引数
    ///
    /// * `mode` - The mode to use from the next call on.
    pub fn set_mode(&mut self, mode: Mode) {
        self.segmenter.mode = mode;
    }

    /// Sets whether whitespace tokens are kept in the output for
    /// subsequent calls.
    ///
    /// # 引数
    ///
    /// * `keep` - `true` to keep whitespace tokens, `false` (MeCab-compatible
    ///   default) to drop them.
    pub fn set_keep_whitespace(&mut self, keep: bool) {
        self.segmenter.keep_whitespace = keep;
    }

    /// Returns a shared reference to the underlying segmenter.
    ///
    /// No `&mut` accessor is provided on purpose: swapping the dictionary
    /// out from under the reused lattice would break the dictionary↔lattice
    /// pairing this type exists to guarantee.
    ///
    /// # 戻り値
    ///
    /// The segmenter this worker is bound to.
    pub fn segmenter(&self) -> &Segmenter {
        &self.segmenter
    }

    /// Immediately shrinks the worker's internal buffers to what an input
    /// of `text_len_hint` bytes needs, and resets the automatic-shrink
    /// window.
    ///
    /// # 引数
    ///
    /// * `text_len_hint` - Expected typical input length in bytes.
    pub fn shrink_to(&mut self, text_len_hint: usize) {
        self.lattice
            .shrink_to(text_len_hint.min(MAX_SENTENCE_BYTES));
        self.offsets.shrink_to_fit();
        self.window_max_needed = 0;
        self.calls_in_window = 0;
    }

    /// Discards the internal buffers, replacing them with fresh ones.
    ///
    /// Intended for recovery paths (e.g. after a panic poisoned a mutex
    /// holding this worker) where the buffers may hold an inconsistent
    /// intermediate state; the segmenter configuration is preserved.
    pub fn reset(&mut self) {
        self.lattice = Lattice::default();
        self.offsets = Vec::new();
        self.window_max_needed = 0;
        self.calls_in_window = 0;
    }

    /// Records one call of `text_len` bytes and, once per shrink window,
    /// shrinks the lattice if its capacity exceeds the window's observed
    /// need by more than the hysteresis factor.
    ///
    /// The per-call need is capped at [`MAX_SENTENCE_BYTES`] because the
    /// segmenter never feeds the lattice a longer sentence; using the full
    /// text length would only over-estimate (which is the safe direction —
    /// it can at most delay a shrink, never cause one too early).
    ///
    /// # 引数
    ///
    /// * `text_len` - Length in bytes of the current call's input.
    fn maybe_auto_shrink(&mut self, text_len: usize) {
        self.window_max_needed = self.window_max_needed.max(text_len.min(MAX_SENTENCE_BYTES));
        self.calls_in_window += 1;
        if self.calls_in_window >= SHRINK_WINDOW_CALLS {
            let target = self.window_max_needed.max(SHRINK_FLOOR_BYTES);
            if self.lattice.capacity() > target.saturating_mul(SHRINK_HYSTERESIS) {
                self.lattice.shrink_to(target);
            }
            self.window_max_needed = 0;
            self.calls_in_window = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    mod with_ipadic {
        use std::borrow::Cow;

        use lindera_dictionary::mode::{Mode, Penalty};

        use crate::dictionary::load_dictionary;
        use crate::segmenter::Segmenter;

        fn ipadic_segmenter() -> Segmenter {
            let dictionary = match load_dictionary("embedded://ipadic") {
                Ok(dictionary) => dictionary,
                Err(err) => panic!("failed to load embedded IPADIC: {err}"),
            };
            Segmenter::new(Mode::Normal, dictionary, None)
        }

        #[test]
        fn test_worker_matches_segment_output() {
            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();

            let texts = [
                "すもももももももものうち",
                "関西国際空港限定トートバッグ",
                "Lindera is a morphological analysis library.",
                "",
                "検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報を検索する機能およびそのプログラム。",
            ];

            // Repeat the whole corpus to exercise buffer reuse across calls.
            for _ in 0..3 {
                for text in texts {
                    let expected = match segmenter.segment(Cow::Borrowed(text)) {
                        Ok(tokens) => tokens,
                        Err(err) => panic!("segment failed: {err}"),
                    };
                    let actual = match worker.segment(text) {
                        Ok(tokens) => tokens,
                        Err(err) => panic!("worker.segment failed: {err}"),
                    };
                    assert_eq!(expected.len(), actual.len(), "token count for {text:?}");
                    for (e, a) in expected.iter().zip(actual.iter()) {
                        assert_eq!(e.surface, a.surface);
                        assert_eq!(e.byte_start, a.byte_start);
                        assert_eq!(e.byte_end, a.byte_end);
                        assert_eq!(e.position, a.position);
                        assert_eq!(e.word_id, a.word_id);
                    }
                }
            }
        }

        #[test]
        fn test_worker_matches_segment_nbest_output() {
            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();
            let text = "すもももももももものうち";

            let expected = match segmenter.segment_nbest(Cow::Borrowed(text), 5, false, None) {
                Ok(results) => results,
                Err(err) => panic!("segment_nbest failed: {err}"),
            };
            // Second call exercises nbest lattice reuse.
            for _ in 0..2 {
                let actual = match worker.segment_nbest(text, 5, false, None) {
                    Ok(results) => results,
                    Err(err) => panic!("worker.segment_nbest failed: {err}"),
                };
                assert_eq!(expected.len(), actual.len());
                for ((e_tokens, e_cost), (a_tokens, a_cost)) in expected.iter().zip(actual.iter()) {
                    assert_eq!(e_cost, a_cost);
                    let e_surfaces: Vec<_> = e_tokens.iter().map(|t| t.surface.clone()).collect();
                    let a_surfaces: Vec<_> = a_tokens.iter().map(|t| t.surface.clone()).collect();
                    assert_eq!(e_surfaces, a_surfaces);
                }
            }
        }

        #[test]
        fn test_worker_mode_and_whitespace_switching() {
            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();
            let text = "関西国際空港へ 行く";

            // Decompose mode must match a segmenter configured the same way.
            worker.set_mode(Mode::Decompose(Penalty::default()));
            let decompose_segmenter = Segmenter::new(
                Mode::Decompose(Penalty::default()),
                segmenter.dictionary.clone(),
                None,
            );
            let expected = match decompose_segmenter.segment(Cow::Borrowed(text)) {
                Ok(tokens) => tokens,
                Err(err) => panic!("segment failed: {err}"),
            };
            let actual = match worker.segment(text) {
                Ok(tokens) => tokens,
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            let expected_surfaces: Vec<_> = expected.iter().map(|t| t.surface.clone()).collect();
            let actual_surfaces: Vec<_> = actual.iter().map(|t| t.surface.clone()).collect();
            assert_eq!(expected_surfaces, actual_surfaces);

            // Switching back restores Normal-mode output.
            worker.set_mode(Mode::Normal);
            let normal = match worker.segment(text) {
                Ok(tokens) => tokens,
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            let fresh = match segmenter.segment(Cow::Borrowed(text)) {
                Ok(tokens) => tokens,
                Err(err) => panic!("segment failed: {err}"),
            };
            assert_eq!(normal.len(), fresh.len());

            // keep_whitespace=true must surface the space token.
            worker.set_keep_whitespace(true);
            let kept = match worker.segment(text) {
                Ok(tokens) => tokens,
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            assert!(
                kept.iter().any(|t| t.surface == " "),
                "whitespace token missing with keep_whitespace=true"
            );
            worker.set_keep_whitespace(false);
        }

        /// The automatic policy must release a lattice grown by one long
        /// delimiter-free sentence once a full window of short inputs shows
        /// the capacity is oversized (target = max(window need, floor);
        /// shrink when capacity > hysteresis * target).
        #[test]
        fn test_auto_shrink_fires_after_window_of_short_inputs() {
            use super::super::{SHRINK_FLOOR_BYTES, SHRINK_HYSTERESIS, SHRINK_WINDOW_CALLS};

            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();

            // One delimiter-free sentence well above the shrink threshold
            // (but under the 32 KiB forced-cut bound): 7000 * 3 bytes.
            let long_text = "あ".repeat(7000);
            assert!(long_text.len() > SHRINK_FLOOR_BYTES * SHRINK_HYSTERESIS);
            if let Err(err) = worker.segment(&long_text) {
                panic!("worker.segment failed: {err}");
            }
            assert!(
                worker.lattice.capacity() >= long_text.len(),
                "long sentence did not grow the lattice"
            );

            // Two full windows of short inputs: the first window still
            // contains the long call in its max, the second one triggers
            // the shrink down to the floor.
            let short_text = "すもももももももものうち";
            for _ in 0..(2 * SHRINK_WINDOW_CALLS + 1) {
                if let Err(err) = worker.segment(short_text) {
                    panic!("worker.segment failed: {err}");
                }
            }
            assert!(
                worker.lattice.capacity() <= SHRINK_FLOOR_BYTES,
                "auto shrink did not fire: capacity {} > floor {}",
                worker.lattice.capacity(),
                SHRINK_FLOOR_BYTES
            );

            // Output stays correct after the shrink.
            let tokens = match worker.segment(short_text) {
                Ok(tokens) => tokens,
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            assert!(!tokens.is_empty());
        }

        #[test]
        fn test_worker_shrink_to_keeps_output_identical() {
            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();
            let text = "すもももももももものうち";

            let before = match worker.segment(text) {
                Ok(tokens) => tokens.len(),
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            worker.shrink_to(0);
            let after = match worker.segment(text) {
                Ok(tokens) => tokens.len(),
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            assert_eq!(before, after, "output changed after shrink_to");
        }
    }
}
