use lindera::LinderaResult;
use lindera::mode::Mode;
use lindera::token::Token;
use lindera::worker::SegmentWorker;

use crate::character_filter::{BoxCharacterFilter, OffsetMapping};
use crate::token_filter::BoxTokenFilter;
use crate::tokenizer::{Tokenizer, correct_offsets};

/// A reusable tokenization session over a [`Tokenizer`]'s full analysis
/// chain (character filters → segmentation → token filters), owning every
/// per-call buffer: the Viterbi lattice and backtrace scratch (via
/// [`SegmentWorker`]), the normalized-text buffer the character filters
/// operate on, and the offset-mapping scratch.
///
/// Compared to [`Tokenizer::tokenize`], repeated calls avoid the fresh
/// lattice allocation, and — when character filters are configured — both
/// the per-call `String` promotion of the input and the per-token surface
/// `String` allocations (surfaces borrow the worker's buffer instead).
///
/// Create a worker with [`Tokenizer::new_worker`] or
/// [`Tokenizer::into_worker`]; it stays permanently bound to that
/// tokenizer's dictionary and filter chain. Returned tokens borrow the
/// worker, so they must be consumed before the next call — the usual
/// per-line loop compiles as-is:
///
/// ```ignore
/// let mut worker = tokenizer.new_worker();
/// for line in lines {
///     let tokens = worker.tokenize(line)?;
///     for token in &tokens {
///         // use token.surface, ...
///     }
/// }
/// ```
///
/// The worker is `Send + Sync`; the `&mut self` API means cross-thread
/// sharing requires external synchronization (one worker per thread, or a
/// `Mutex` as `lindera-binding-core` does).
pub struct AnalysisWorker {
    /// Character filters applied to the text before segmentation.
    character_filters: Vec<BoxCharacterFilter>,
    /// Token filters applied to the tokens after segmentation.
    token_filters: Vec<BoxTokenFilter>,
    /// The reusable segmentation session (lattice + backtrace scratch).
    segment_worker: SegmentWorker,
    /// Persistent buffer holding the character-filtered (normalized) text;
    /// token surfaces borrow from it when character filters are configured.
    text_buf: String,
    /// Scratch for the per-call offset mappings produced by the character
    /// filters (the `Vec` is reused; mappings themselves are per-call).
    offset_mappings: Vec<OffsetMapping>,
}

impl Tokenizer {
    /// Creates a reusable [`AnalysisWorker`] bound to a deep clone of this
    /// tokenizer (filters are cloned via `box_clone`; the dictionary clone
    /// is `Arc`-cheap, but a configured user dictionary is deep-copied —
    /// prefer [`Tokenizer::into_worker`] when the tokenizer itself is no
    /// longer needed).
    ///
    /// # 戻り値
    ///
    /// A fresh worker with empty internal buffers.
    pub fn new_worker(&self) -> AnalysisWorker {
        self.clone().into_worker()
    }

    /// Consumes this tokenizer and creates a reusable [`AnalysisWorker`]
    /// from it without cloning any of its parts.
    ///
    /// # 戻り値
    ///
    /// A fresh worker with empty internal buffers.
    pub fn into_worker(self) -> AnalysisWorker {
        AnalysisWorker {
            character_filters: self.character_filters,
            token_filters: self.token_filters,
            segment_worker: self.segmenter.into_worker(),
            text_buf: String::new(),
            offset_mappings: Vec::new(),
        }
    }
}

impl AnalysisWorker {
    /// Tokenizes `text` through the full analysis chain, reusing this
    /// worker's internal buffers.
    ///
    /// Produces exactly the same tokens as [`Tokenizer::tokenize`] for the
    /// same input and configuration. The returned tokens borrow both
    /// `text` and this worker, so they must be consumed before the next
    /// call.
    ///
    /// # 引数
    ///
    /// * `text` - The input text to tokenize.
    ///
    /// # 戻り値
    ///
    /// The filtered tokens, in reading order, with byte offsets corrected
    /// back to the original (pre-filter) text.
    pub fn tokenize<'w>(&'w mut self, text: &'w str) -> LinderaResult<Vec<Token<'w>>> {
        let Self {
            character_filters,
            token_filters,
            segment_worker,
            text_buf,
            offset_mappings,
        } = self;
        offset_mappings.clear();

        let (mut tokens, final_text_len) = if character_filters.is_empty() {
            (segment_worker.segment(text)?, text.len())
        } else {
            text_buf.clear();
            text_buf.push_str(text);
            for character_filter in character_filters.iter() {
                let mapping = character_filter.apply(text_buf)?;
                if !mapping.is_empty() {
                    offset_mappings.push(mapping);
                }
            }
            // Read the length before segmenting: the tokens returned below
            // hold a shared borrow of `text_buf` for the rest of the call.
            let final_text_len = text_buf.len();
            (segment_worker.segment(text_buf.as_str())?, final_text_len)
        };

        for token_filter in token_filters.iter() {
            token_filter.apply(&mut tokens)?;
        }

        correct_offsets(&mut tokens, offset_mappings, final_text_len);

        Ok(tokens)
    }

    /// Tokenizes `text` and returns the top-N results with costs, through
    /// the full analysis chain, reusing this worker's internal buffers.
    ///
    /// Produces exactly the same results as [`Tokenizer::tokenize_nbest`]
    /// for the same input and configuration.
    ///
    /// # 引数
    ///
    /// * `text` - The input text to tokenize.
    /// * `n` - Maximum number of tokenizations to return.
    /// * `unique` - Deduplicate results with identical word boundaries.
    /// * `cost_threshold` - Discard paths costing more than best + threshold.
    ///
    /// # 戻り値
    ///
    /// Up to `n` `(tokens, cost)` pairs ordered by cost (best first).
    pub fn tokenize_nbest<'w>(
        &'w mut self,
        text: &'w str,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> LinderaResult<Vec<(Vec<Token<'w>>, i64)>> {
        let Self {
            character_filters,
            token_filters,
            segment_worker,
            text_buf,
            offset_mappings,
        } = self;
        offset_mappings.clear();

        let (mut all_results, final_text_len) = if character_filters.is_empty() {
            (
                segment_worker.segment_nbest(text, n, unique, cost_threshold)?,
                text.len(),
            )
        } else {
            text_buf.clear();
            text_buf.push_str(text);
            for character_filter in character_filters.iter() {
                let mapping = character_filter.apply(text_buf)?;
                if !mapping.is_empty() {
                    offset_mappings.push(mapping);
                }
            }
            let final_text_len = text_buf.len();
            (
                segment_worker.segment_nbest(text_buf.as_str(), n, unique, cost_threshold)?,
                final_text_len,
            )
        };

        for (tokens, _cost) in &mut all_results {
            for token_filter in token_filters.iter() {
                token_filter.apply(tokens)?;
            }
            correct_offsets(tokens, offset_mappings, final_text_len);
        }

        Ok(all_results)
    }

    /// Sets the segmentation mode for subsequent calls.
    ///
    /// # 引数
    ///
    /// * `mode` - The mode to use from the next call on.
    pub fn set_mode(&mut self, mode: Mode) {
        self.segment_worker.set_mode(mode);
    }

    /// Sets whether whitespace tokens are kept in the output for
    /// subsequent calls.
    ///
    /// # 引数
    ///
    /// * `keep` - `true` to keep whitespace tokens, `false` to drop them.
    pub fn set_keep_whitespace(&mut self, keep: bool) {
        self.segment_worker.set_keep_whitespace(keep);
    }

    /// Immediately shrinks the worker's internal buffers to what an input
    /// of `text_len_hint` bytes needs.
    ///
    /// # 引数
    ///
    /// * `text_len_hint` - Expected typical input length in bytes.
    pub fn shrink_to(&mut self, text_len_hint: usize) {
        self.segment_worker.shrink_to(text_len_hint);
        self.text_buf.shrink_to(text_len_hint);
        self.offset_mappings.shrink_to_fit();
    }

    /// Discards all internal buffers, replacing them with fresh ones.
    ///
    /// Intended for recovery paths (e.g. after a panic poisoned a mutex
    /// holding this worker) where the buffers may be in an inconsistent
    /// intermediate state; configuration (dictionary, filters, mode) is
    /// preserved.
    pub fn reset(&mut self) {
        self.segment_worker.reset();
        self.text_buf = String::new();
        self.offset_mappings = Vec::new();
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    mod with_ipadic {
        use std::collections::HashMap;

        use lindera::dictionary::load_dictionary;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;

        use crate::character_filter::BoxCharacterFilter;
        use crate::character_filter::mapping::MappingCharacterFilter;
        use crate::token_filter::BoxTokenFilter;
        use crate::token_filter::lowercase::LowercaseTokenFilter;
        use crate::tokenizer::Tokenizer;

        fn ipadic_tokenizer(with_filters: bool) -> Tokenizer {
            let dictionary = match load_dictionary("embedded://ipadic") {
                Ok(dictionary) => dictionary,
                Err(err) => panic!("failed to load embedded IPADIC: {err}"),
            };
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let mut tokenizer = Tokenizer::new(segmenter);
            if with_filters {
                let mut mapping = HashMap::new();
                mapping.insert("リンデラ".to_string(), "Lindera".to_string());
                let filter = match MappingCharacterFilter::new(mapping) {
                    Ok(filter) => filter,
                    Err(err) => panic!("failed to build mapping filter: {err}"),
                };
                tokenizer
                    .character_filters
                    .push(BoxCharacterFilter::from(filter));
                tokenizer
                    .token_filters
                    .push(BoxTokenFilter::from(LowercaseTokenFilter::new()));
            }
            tokenizer
        }

        fn assert_same_tokens(
            tokenizer: &Tokenizer,
            worker_tokens: &[(String, usize, usize)],
            text: &str,
        ) {
            let expected = match tokenizer.tokenize(text) {
                Ok(tokens) => tokens,
                Err(err) => panic!("tokenize failed: {err}"),
            };
            let expected: Vec<(String, usize, usize)> = expected
                .iter()
                .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                .collect();
            assert_eq!(expected, worker_tokens, "mismatch for {text:?}");
        }

        #[test]
        fn test_worker_matches_tokenize_without_filters() {
            let tokenizer = ipadic_tokenizer(false);
            let mut worker = tokenizer.new_worker();

            let texts = [
                "すもももももももものうち",
                "関西国際空港限定トートバッグ",
                "",
            ];
            for _ in 0..3 {
                for text in texts {
                    let actual: Vec<(String, usize, usize)> = {
                        let tokens = match worker.tokenize(text) {
                            Ok(tokens) => tokens,
                            Err(err) => panic!("worker.tokenize failed: {err}"),
                        };
                        tokens
                            .iter()
                            .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                            .collect()
                    };
                    assert_same_tokens(&tokenizer, &actual, text);
                }
            }
        }

        #[test]
        fn test_worker_matches_tokenize_with_filters() {
            let tokenizer = ipadic_tokenizer(true);
            let mut worker = tokenizer.new_worker();

            // The mapping filter rewrites リンデラ -> Lindera (changing byte
            // lengths, so offset corrections are exercised), and the
            // lowercase token filter forces owned surfaces.
            let texts = [
                "リンデラは形態素解析器です。",
                "TOKYO と リンデラ",
                "すもももももももものうち",
            ];
            for _ in 0..3 {
                for text in texts {
                    let actual: Vec<(String, usize, usize)> = {
                        let tokens = match worker.tokenize(text) {
                            Ok(tokens) => tokens,
                            Err(err) => panic!("worker.tokenize failed: {err}"),
                        };
                        tokens
                            .iter()
                            .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                            .collect()
                    };
                    assert_same_tokens(&tokenizer, &actual, text);
                }
            }
        }

        #[test]
        fn test_worker_matches_tokenize_nbest() {
            let tokenizer = ipadic_tokenizer(true);
            let mut worker = tokenizer.new_worker();
            let text = "リンデラとすもももももももものうち";

            let expected = match tokenizer.tokenize_nbest(text, 3, false, None) {
                Ok(results) => results,
                Err(err) => panic!("tokenize_nbest failed: {err}"),
            };
            for _ in 0..2 {
                let actual = match worker.tokenize_nbest(text, 3, false, None) {
                    Ok(results) => results,
                    Err(err) => panic!("worker.tokenize_nbest failed: {err}"),
                };
                assert_eq!(expected.len(), actual.len());
                for ((e_tokens, e_cost), (a_tokens, a_cost)) in expected.iter().zip(actual.iter()) {
                    assert_eq!(e_cost, a_cost);
                    let e: Vec<(String, usize, usize)> = e_tokens
                        .iter()
                        .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                        .collect();
                    let a: Vec<(String, usize, usize)> = a_tokens
                        .iter()
                        .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                        .collect();
                    assert_eq!(e, a);
                }
            }
        }

        #[test]
        fn test_worker_reset_and_shrink_keep_output() {
            let tokenizer = ipadic_tokenizer(true);
            let mut worker = tokenizer.new_worker();
            let text = "リンデラは形態素解析器です。";

            let before: Vec<String> = {
                let tokens = match worker.tokenize(text) {
                    Ok(tokens) => tokens,
                    Err(err) => panic!("worker.tokenize failed: {err}"),
                };
                tokens.iter().map(|t| t.surface.to_string()).collect()
            };

            worker.reset();
            let after_reset: Vec<String> = {
                let tokens = match worker.tokenize(text) {
                    Ok(tokens) => tokens,
                    Err(err) => panic!("worker.tokenize failed: {err}"),
                };
                tokens.iter().map(|t| t.surface.to_string()).collect()
            };
            assert_eq!(before, after_reset, "output changed after reset()");

            worker.shrink_to(0);
            let after_shrink: Vec<String> = {
                let tokens = match worker.tokenize(text) {
                    Ok(tokens) => tokens,
                    Err(err) => panic!("worker.tokenize failed: {err}"),
                };
                tokens.iter().map(|t| t.surface.to_string()).collect()
            };
            assert_eq!(before, after_shrink, "output changed after shrink_to()");
        }
    }
}
