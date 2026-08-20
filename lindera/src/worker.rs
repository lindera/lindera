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
/// factor, so a workload hovering around one size never thrashes. Halved
/// from 4 when the lattice became char-indexed (#943): the longest sentence
/// the segmenter can feed is now 32 Ki slots (ASCII) but only ~10.9 Ki
/// slots for 3-byte CJK, which a 4x margin over the 4 Ki floor would never
/// reach.
const SHRINK_HYSTERESIS: usize = 2;

/// Floor (in slots = characters) below which the lattice is never shrunk.
/// Typical line-oriented input stays under this, so the automatic policy
/// never fires for it; worst-case retention above the floor is bounded to
/// a few MB instead of the ~6 MB a 32 KiB CJK sentence can pin (#943).
const SHRINK_FLOOR_SLOTS: usize = 4 * 1024;

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
/// The worker automatically bounds retained memory: one maximum-length
/// sentence grows the lattice to several MB (~18 MB for 32 Ki ASCII slots,
/// ~6 MB for a 32 KiB CJK sentence's ~10.9 Ki slots), and without
/// intervention that stays pinned for the worker's lifetime. Every [`SHRINK_WINDOW_CALLS`] calls the
/// worker compares the lattice capacity against the window's largest
/// sentence (with a [`SHRINK_HYSTERESIS`]x margin and a [`SHRINK_FLOOR_SLOTS`]
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
    /// Largest per-sentence character count observed in the current shrink
    /// window (drained from the lattice's own accounting).
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
        self.maybe_auto_shrink();
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
        self.maybe_auto_shrink();
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

    /// Sets the unknown-word grouping cap for subsequent calls (MeCab's
    /// `max-grouping-size` semantics; `None` = unbounded, the default).
    ///
    /// # 引数
    ///
    /// * `max_grouping_len` - Maximum grouped characters beyond the first.
    pub fn set_max_grouping_len(&mut self, max_grouping_len: Option<usize>) {
        self.segmenter.max_grouping_len = max_grouping_len;
    }

    /// Enables/disables the unknown-word length ladder for subsequent
    /// calls (see `Segmenter::unknown_word_ladder`; defaults to `true`).
    ///
    /// # 引数
    ///
    /// * `unknown_word_ladder` - Whether to emit the length ladder.
    pub fn set_unknown_word_ladder(&mut self, unknown_word_ladder: bool) {
        self.segmenter.unknown_word_ladder = unknown_word_ladder;
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
    /// The lattice is slot(character)-denominated (#943); a byte length is
    /// a valid conservative bound, since a sentence never has more
    /// characters than bytes.
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

    /// Records one call and, once per shrink window, shrinks the lattice
    /// if its capacity exceeds the window's observed need by more than the
    /// hysteresis factor.
    ///
    /// The per-window need is the largest per-sentence character count the
    /// lattice actually processed ([`Lattice::take_max_char_len`]), so the
    /// accounting stays in the lattice's own slot unit (#943).
    fn maybe_auto_shrink(&mut self) {
        self.window_max_needed = self.window_max_needed.max(self.lattice.take_max_char_len());
        self.calls_in_window += 1;
        if self.calls_in_window >= SHRINK_WINDOW_CALLS {
            let target = self.window_max_needed.max(SHRINK_FLOOR_SLOTS);
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

        /// #944: max_grouping_len caps unknown-word grouping with MeCab
        /// semantics -- a run with more characters beyond the first than
        /// the cap falls back to single-char unknown words -- while None
        /// #945: the length ladder must be additive and switchable. Using
        /// two out-of-vocabulary KANJI characters (char.def: invoke=0,
        /// group=0, length=2 in IPADIC) isolates the ladder from grouping
        /// (which is off for this category) and from max_grouping_len
        /// (irrelevant when group=false): with the ladder on, a 2-char
        /// candidate becomes available and the best path takes it (lower
        /// cost than two isolated 1-char unknown words); with it off,
        /// only 1-char candidates exist, matching pre-#945 output.
        #[test]
        fn test_unknown_word_ladder_toggle() {
            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();

            // Two rare kanji absent from IPADIC's dictionary.
            let text = "龘龍";

            let with_ladder = match worker.segment(text) {
                Ok(tokens) => tokens
                    .iter()
                    .map(|t| t.surface.to_string())
                    .collect::<Vec<_>>(),
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            assert_eq!(
                with_ladder,
                vec![text.to_string()],
                "the ladder's 2-char candidate must win the best path"
            );

            worker.set_unknown_word_ladder(false);
            let without_ladder = match worker.segment(text) {
                Ok(tokens) => tokens
                    .iter()
                    .map(|t| t.surface.to_string())
                    .collect::<Vec<_>>(),
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            assert_eq!(
                without_ladder,
                vec!["龘".to_string(), "龍".to_string()],
                "disabling the ladder must fall back to single-char unknowns"
            );

            worker.set_unknown_word_ladder(true);
            assert_eq!(
                match worker.segment(text) {
                    Ok(tokens) => tokens
                        .iter()
                        .map(|t| t.surface.to_string())
                        .collect::<Vec<_>>(),
                    Err(err) => panic!("worker.segment failed: {err}"),
                },
                with_ladder,
                "re-enabling the ladder must restore the default output"
            );
        }

        /// #945: the length ladder must also fire on the Decompose-mode
        /// path, which reads run lengths from the #944 precomputed buffer
        /// instead of Normal mode's forward scan.
        #[test]
        fn test_unknown_word_ladder_decompose_mode() {
            use lindera_dictionary::mode::Penalty;

            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();
            worker.set_mode(Mode::Decompose(Penalty::default()));

            let text = "龘龍";
            let with_ladder = match worker.segment(text) {
                Ok(tokens) => tokens
                    .iter()
                    .map(|t| t.surface.to_string())
                    .collect::<Vec<_>>(),
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            assert_eq!(with_ladder, vec![text.to_string()]);

            worker.set_unknown_word_ladder(false);
            let without_ladder = match worker.segment(text) {
                Ok(tokens) => tokens
                    .iter()
                    .map(|t| t.surface.to_string())
                    .collect::<Vec<_>>(),
                Err(err) => panic!("worker.segment failed: {err}"),
            };
            assert_eq!(without_ladder, vec!["龘".to_string(), "龍".to_string()]);
        }

        /// (the default) keeps grouping unbounded.
        #[test]
        fn test_max_grouping_len_caps_unknown_grouping() {
            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();
            // Isolate max_grouping_len from the length ladder (#945,
            // default on), which would otherwise also contribute
            // candidates and change the best path independently.
            worker.set_unknown_word_ladder(false);

            // An out-of-vocabulary katakana run (5 chars); IPADIC's
            // KATAKANA category groups, so the default spans the run.
            let text = "ヷヸヹヺヷ";
            let surfaces = |worker: &mut super::super::SegmentWorker| -> Vec<String> {
                match worker.segment(text) {
                    Ok(tokens) => tokens.iter().map(|t| t.surface.to_string()).collect(),
                    Err(err) => panic!("worker.segment failed: {err}"),
                }
            };

            let unbounded = surfaces(&mut worker);
            assert_eq!(
                unbounded,
                vec![text.to_string()],
                "unbounded grouping must span the whole run"
            );

            // With a cap of 2: positions 0 and 1 see runs of 5 and 4
            // chars (3+ beyond the first, over the cap) and fall back to
            // single-char unknowns; position 2's remaining run of 3 chars
            // (2 beyond the first) fits the cap and groups -- the exact
            // MeCab max-grouping-size behavior.
            worker.set_max_grouping_len(Some(2));
            assert_eq!(
                surfaces(&mut worker),
                vec!["ヷ".to_string(), "ヸ".to_string(), "ヹヺヷ".to_string()],
                "cap must reject over-long runs and admit fitting tails"
            );

            // A cap of exactly run-1 admits the grouped candidate.
            worker.set_max_grouping_len(Some(4));
            assert_eq!(surfaces(&mut worker), vec![text.to_string()]);

            // Clearing the cap restores the default behavior.
            worker.set_max_grouping_len(None);
            assert_eq!(surfaces(&mut worker), unbounded);
        }

        /// The automatic policy must release a lattice grown by one long
        /// delimiter-free sentence once a full window of short inputs shows
        /// the capacity is oversized (target = max(window need, floor);
        /// shrink when capacity > hysteresis * target).
        #[test]
        fn test_auto_shrink_fires_after_window_of_short_inputs() {
            use super::super::{SHRINK_FLOOR_SLOTS, SHRINK_HYSTERESIS, SHRINK_WINDOW_CALLS};

            let segmenter = ipadic_segmenter();
            let mut worker = segmenter.new_worker();

            // One delimiter-free sentence well above the shrink threshold
            // in slots = characters (but under the 32 KiB forced-cut
            // bound: 9000 * 3 bytes = 27 KB).
            let long_chars = 9000;
            let long_text = "あ".repeat(long_chars);
            assert!(long_chars > SHRINK_FLOOR_SLOTS * SHRINK_HYSTERESIS);
            if let Err(err) = worker.segment(&long_text) {
                panic!("worker.segment failed: {err}");
            }
            assert!(
                worker.lattice.capacity() >= long_chars,
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
                worker.lattice.capacity() <= SHRINK_FLOOR_SLOTS,
                "auto shrink did not fire: capacity {} > floor {}",
                worker.lattice.capacity(),
                SHRINK_FLOOR_SLOTS
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
