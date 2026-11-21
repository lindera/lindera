//! Tokenization of a list of texts on several threads.
//!
//! An [`AnalysisWorkerPool`] holds [`AnalysisWorker`]s with one
//! configuration and tokenizes every text of a list as a call of its own,
//! on up to a given number of threads. The results come back in the order of
//! the texts, and they are those of one worker that tokenizes the texts one
//! after the other: a call does not depend on the calls before it, and the
//! workers are copies of one configuration.

use std::any::Any;
use std::num::NonZeroUsize;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use lindera_segmenter::LinderaResult;
use lindera_segmenter::error::{LinderaError, LinderaErrorKind};
use lindera_segmenter::token::Token;

use crate::tokenizer::Tokenizer;
use crate::worker::AnalysisWorker;

/// The input, in bytes, that one more thread needs: a batch of fewer bytes
/// than this per thread runs on fewer threads, and a short batch on the
/// calling thread alone. On the 16-core machine of #1402's measurements,
/// starting a thread took about 10 µs, and tokenizing 16 KiB took 0.6 ms
/// (IPADIC) to 1.7 ms (ko-dic), so the start costs at most a few percent of
/// a thread's work. Affects only the speed.
const MIN_BYTES_PER_THREAD: usize = 16 * 1024;

/// The chunks of a batch per thread. The threads take the chunks one at a
/// time, so more chunks balance texts of uneven lengths better, and fewer
/// chunks cost fewer atomic operations. Affects only the speed.
const CHUNKS_PER_THREAD: usize = 8;

/// [`AnalysisWorker`]s with one configuration, which tokenize a list of
/// texts on several threads.
///
/// Every text is tokenized as a call of its own, as
/// [`AnalysisWorker::tokenize`] tokenizes it, and the results come back in
/// the order of the texts: they are the same for every number of threads.
/// A list of the lines of a document gives the same tokens as tokenizing
/// the lines one by one, as `lindera tokenize` does; it does not give those
/// of the whole document as one text, whose offsets, positions and N-best
/// results span its lines.
///
/// The pool starts with one worker and makes more from it (with
/// [`AnalysisWorker::to_tokenizer`]) when a call runs on more threads than
/// it has workers; each worker keeps its buffers for the next calls, and
/// each holds its own copy of the user dictionary, if any.
/// [`Self::truncate_workers`] lets the extra workers go.
///
/// A call runs on the calling thread alone when it is given one thread, one
/// text, or a short input, and always on wasm, which has no threads.
/// Otherwise it starts threads with [`std::thread::scope`], and the calling
/// thread works with them.
///
/// Create a pool with [`Tokenizer::new_worker_pool`] or
/// [`Tokenizer::into_worker_pool`].
pub struct AnalysisWorkerPool {
    /// The first worker, which the pool is made with and which the others
    /// are made from.
    first: AnalysisWorker,
    /// The workers made from the first one, as the calls need them.
    others: Vec<AnalysisWorker>,
}

impl Tokenizer {
    /// Creates an [`AnalysisWorkerPool`] bound to a deep clone of this
    /// tokenizer (see [`Tokenizer::new_worker`]).
    ///
    /// # Returns
    ///
    /// A pool with one worker.
    pub fn new_worker_pool(&self) -> AnalysisWorkerPool {
        self.clone().into_worker_pool()
    }

    /// Consumes this tokenizer and creates an [`AnalysisWorkerPool`] from it
    /// (see [`Tokenizer::into_worker`]).
    ///
    /// # Returns
    ///
    /// A pool with one worker.
    pub fn into_worker_pool(self) -> AnalysisWorkerPool {
        AnalysisWorkerPool {
            first: self.into_worker(),
            others: Vec::new(),
        }
    }
}

impl AnalysisWorkerPool {
    /// Tokenizes every text of `texts` as [`AnalysisWorker::tokenize`]
    /// does, on up to `max_threads` threads, and turns the tokens of each
    /// text into a result with `f`.
    ///
    /// `f` runs on the thread that tokenized the text, as soon as it is
    /// tokenized: the tokens borrow that thread's worker, so whatever the
    /// caller keeps of them has to be owned (`String`s, counts, a formatted
    /// output).
    ///
    /// The error of the first text in input order that fails is returned,
    /// as a loop over the texts returns it; the texts after it may or may
    /// not have been tokenized. A panic in the tokenization or in `f` is
    /// resumed on the calling thread in the same way, after the worker that
    /// panicked is reset, so the pool stays usable.
    ///
    /// # Arguments
    ///
    /// * `texts` - The texts, each tokenized as a call of its own.
    /// * `max_threads` - The most threads to run on, the calling thread
    ///   included.
    /// * `f` - Turns the tokens of one text into its result.
    ///
    /// # Returns
    ///
    /// The results of `f`, in the order of `texts`.
    pub fn tokenize_batch<S, F, R>(
        &mut self,
        texts: &[S],
        max_threads: NonZeroUsize,
        f: F,
    ) -> LinderaResult<Vec<R>>
    where
        S: AsRef<str> + Sync,
        F: for<'t> Fn(Vec<Token<'t>>) -> R + Sync,
        R: Send,
    {
        self.run(texts, max_threads, MIN_BYTES_PER_THREAD, |worker, text| {
            worker.tokenize(text).map(&f)
        })
    }

    /// Tokenizes every text of `texts` as [`AnalysisWorker::tokenize_nbest`]
    /// does, on up to `max_threads` threads, and turns the results of each
    /// text into a result with `f`, as [`Self::tokenize_batch`] does.
    ///
    /// # Arguments
    ///
    /// * `texts` - The texts, each tokenized as a call of its own.
    /// * `n` - Maximum number of tokenizations per text.
    /// * `unique` - Deduplicate results with identical word boundaries.
    /// * `cost_threshold` - Discard results costing more than the best
    ///   result of their text + threshold.
    /// * `max_threads` - The most threads to run on, the calling thread
    ///   included.
    /// * `f` - Turns the `(tokens, cost)` pairs of one text into its result.
    ///
    /// # Returns
    ///
    /// The results of `f`, in the order of `texts`.
    pub fn tokenize_nbest_batch<S, F, R>(
        &mut self,
        texts: &[S],
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
        max_threads: NonZeroUsize,
        f: F,
    ) -> LinderaResult<Vec<R>>
    where
        S: AsRef<str> + Sync,
        F: for<'t> Fn(Vec<(Vec<Token<'t>>, i64)>) -> R + Sync,
        R: Send,
    {
        self.run(texts, max_threads, MIN_BYTES_PER_THREAD, |worker, text| {
            worker
                .tokenize_nbest(text, n, unique, cost_threshold)
                .map(&f)
        })
    }

    /// Returns the number of workers the pool holds.
    ///
    /// # Returns
    ///
    /// 1 for a new pool, and up to the most threads a call has run on.
    pub fn worker_count(&self) -> usize {
        1 + self.others.len()
    }

    /// Drops the workers beyond `max_workers`, with their buffers and their
    /// copies of the user dictionary. A later call makes them again if it
    /// needs them.
    ///
    /// # Arguments
    ///
    /// * `max_workers` - The most workers to keep.
    pub fn truncate_workers(&mut self, max_workers: NonZeroUsize) {
        self.others.truncate(max_workers.get() - 1);
    }

    /// Immediately shrinks every worker's internal buffers to what an input
    /// of `text_len_hint` bytes needs (see [`AnalysisWorker::shrink_to`]).
    ///
    /// # Arguments
    ///
    /// * `text_len_hint` - Expected typical input length in bytes.
    pub fn shrink_to(&mut self, text_len_hint: usize) {
        self.first.shrink_to(text_len_hint);
        for worker in &mut self.others {
            worker.shrink_to(text_len_hint);
        }
    }

    /// Discards every worker's internal buffers, replacing them with fresh
    /// ones (see [`AnalysisWorker::reset`]).
    pub fn reset(&mut self) {
        self.first.reset();
        for worker in &mut self.others {
            worker.reset();
        }
    }

    /// Runs `job` on every text, on as many threads as [`thread_count`]
    /// gives, and returns its results in input order.
    ///
    /// # Arguments
    ///
    /// * `texts` - The texts.
    /// * `max_threads` - The most threads to run on.
    /// * `min_bytes_per_thread` - The input one more thread needs.
    /// * `job` - Tokenizes one text with a worker and makes its result.
    ///
    /// # Returns
    ///
    /// The results, in the order of `texts`, or the error of the first text
    /// in input order that fails.
    fn run<S, R, J>(
        &mut self,
        texts: &[S],
        max_threads: NonZeroUsize,
        min_bytes_per_thread: usize,
        job: J,
    ) -> LinderaResult<Vec<R>>
    where
        S: AsRef<str> + Sync,
        R: Send,
        J: Fn(&mut AnalysisWorker, &str) -> LinderaResult<R> + Sync,
    {
        let threads = thread_count(texts, max_threads, min_bytes_per_thread);
        if threads == 1 {
            return run_in_order(&mut self.first, texts, &job);
        }
        self.add_workers(threads);

        let chunk_len = texts.len().div_ceil(threads * CHUNKS_PER_THREAD);
        let batch = Batch {
            texts,
            chunk_len,
            chunks: texts.len().div_ceil(chunk_len),
            next_chunk: AtomicUsize::new(0),
            stop_at: AtomicUsize::new(usize::MAX),
            job: &job,
        };
        let batch = &batch;
        let outcomes = thread::scope(|scope| {
            let mut handles = Vec::with_capacity(threads - 1);
            for (index, worker) in self.others.iter_mut().take(threads - 1).enumerate() {
                let spawned = thread::Builder::new()
                    .name(format!("lindera-analysis-{}", index + 1))
                    .spawn_scoped(scope, move || batch.drain(worker));
                // A thread that cannot be started leaves its share to the
                // others: the chunks are handed out on demand.
                if let Ok(handle) = spawned {
                    handles.push(handle);
                }
            }
            let mut outcomes = batch.drain(&mut self.first);
            for handle in handles {
                outcomes.extend(join_or_resume(handle));
            }
            outcomes
        });
        in_input_order(texts.len(), outcomes)
    }

    /// Makes workers from the first one until the pool has `count`.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of workers the pool needs.
    fn add_workers(&mut self, count: usize) {
        while self.worker_count() < count {
            let worker = self.first.to_tokenizer().into_worker();
            self.others.push(worker);
        }
    }
}

/// Returns the number of threads a batch runs on: `max_threads`, but no more
/// than one per text, no more than one per [`MIN_BYTES_PER_THREAD`]-sized
/// share of the input, and one on wasm.
///
/// # Arguments
///
/// * `texts` - The texts of the batch.
/// * `max_threads` - The most threads the caller allows.
/// * `min_bytes_per_thread` - The input one more thread needs; 0 lifts this
///   limit.
///
/// # Returns
///
/// The number of threads, 1 or more.
fn thread_count<S: AsRef<str>>(
    texts: &[S],
    max_threads: NonZeroUsize,
    min_bytes_per_thread: usize,
) -> usize {
    if cfg!(target_family = "wasm") {
        return 1;
    }
    let by_size = match min_bytes_per_thread {
        0 => usize::MAX,
        min => texts.iter().map(|text| text.as_ref().len()).sum::<usize>() / min,
    };
    max_threads.get().min(texts.len()).min(by_size).max(1)
}

/// How the work on one text ended.
enum Outcome<R> {
    /// The text's result.
    Done(R),
    /// The text's tokenization failed.
    Failed(LinderaError),
    /// The text's tokenization, or the caller's function, panicked.
    Panicked(Box<dyn Any + Send>),
}

/// Runs `job` on one text, catching a panic and resetting the worker after
/// one, since a panic can leave its buffers in an intermediate state.
///
/// # Arguments
///
/// * `worker` - The worker.
/// * `text` - The text.
/// * `job` - Tokenizes the text with the worker and makes its result.
///
/// # Returns
///
/// How the work on the text ended.
fn run_one<R, J>(worker: &mut AnalysisWorker, text: &str, job: &J) -> Outcome<R>
where
    J: Fn(&mut AnalysisWorker, &str) -> LinderaResult<R>,
{
    match panic::catch_unwind(AssertUnwindSafe(|| job(worker, text))) {
        Ok(Ok(result)) => Outcome::Done(result),
        Ok(Err(err)) => Outcome::Failed(err),
        Err(payload) => {
            worker.reset();
            Outcome::Panicked(payload)
        }
    }
}

/// Runs `job` on every text in order on the calling thread, stopping at the
/// first failure.
///
/// # Arguments
///
/// * `worker` - The worker.
/// * `texts` - The texts.
/// * `job` - Tokenizes one text with the worker and makes its result.
///
/// # Returns
///
/// The results, in the order of `texts`, or the first error; the first
/// panic is resumed.
fn run_in_order<S, R, J>(worker: &mut AnalysisWorker, texts: &[S], job: &J) -> LinderaResult<Vec<R>>
where
    S: AsRef<str>,
    J: Fn(&mut AnalysisWorker, &str) -> LinderaResult<R>,
{
    let mut results = Vec::with_capacity(texts.len());
    for text in texts {
        match run_one(worker, text.as_ref(), job) {
            Outcome::Done(result) => results.push(result),
            Outcome::Failed(err) => return Err(err),
            Outcome::Panicked(payload) => panic::resume_unwind(payload),
        }
    }
    Ok(results)
}

/// A batch shared by the threads that work on it.
struct Batch<'a, S, J> {
    /// The texts.
    texts: &'a [S],
    /// The number of texts per chunk, the last chunk excepted.
    chunk_len: usize,
    /// The number of chunks.
    chunks: usize,
    /// The next chunk to hand out.
    next_chunk: AtomicUsize,
    /// The index of the first text in input order seen to fail so far, or
    /// `usize::MAX`.
    stop_at: AtomicUsize,
    /// Tokenizes one text with a worker and makes its result.
    job: &'a J,
}

impl<S: AsRef<str>, J> Batch<'_, S, J> {
    /// Works on the batch's chunks with `worker` until none is left, or
    /// until the chunks left all come after a failure.
    ///
    /// A text before the first failing text in input order is never
    /// skipped: the chunks are handed out in input order, and `stop_at`
    /// only ever holds the index of a text that failed, so the first
    /// failing text is always tried and every failure is recorded. The
    /// orderings are relaxed: the counter only hands out chunk indices,
    /// `stop_at` only lets work after a failure be skipped, and the
    /// outcomes reach the caller through the threads' join.
    ///
    /// # Arguments
    ///
    /// * `worker` - The worker of the thread.
    ///
    /// # Returns
    ///
    /// The outcome of every text this thread worked on, with its index.
    fn drain<R>(&self, worker: &mut AnalysisWorker) -> Vec<(usize, Outcome<R>)>
    where
        J: Fn(&mut AnalysisWorker, &str) -> LinderaResult<R>,
    {
        let mut outcomes = Vec::new();
        loop {
            let chunk = self.next_chunk.fetch_add(1, Ordering::Relaxed);
            if chunk >= self.chunks {
                break;
            }
            let start = chunk * self.chunk_len;
            // The later chunks start later still.
            if start >= self.stop_at.load(Ordering::Relaxed) {
                break;
            }
            let end = (start + self.chunk_len).min(self.texts.len());
            let Some(texts) = self.texts.get(start..end) else {
                break;
            };
            for (index, text) in (start..end).zip(texts) {
                if index >= self.stop_at.load(Ordering::Relaxed) {
                    break;
                }
                let outcome = run_one(worker, text.as_ref(), self.job);
                let failed = !matches!(outcome, Outcome::Done(_));
                outcomes.push((index, outcome));
                if failed {
                    self.stop_at.fetch_min(index, Ordering::Relaxed);
                    break;
                }
            }
        }
        outcomes
    }
}

/// Waits for a thread and resumes its panic on the calling thread.
///
/// # Arguments
///
/// * `handle` - The thread's handle.
///
/// # Returns
///
/// The thread's return value.
fn join_or_resume<T>(handle: thread::ScopedJoinHandle<'_, T>) -> T {
    match handle.join() {
        Ok(value) => value,
        Err(payload) => panic::resume_unwind(payload),
    }
}

/// Puts the outcomes of a batch back in input order.
///
/// # Arguments
///
/// * `len` - The number of texts.
/// * `outcomes` - The outcome of every text worked on, with its index, in
///   any order.
///
/// # Returns
///
/// The results in input order, or the error of the first failing text in
/// input order; the panic of the first failing text is resumed.
fn in_input_order<R>(len: usize, outcomes: Vec<(usize, Outcome<R>)>) -> LinderaResult<Vec<R>> {
    let mut slots: Vec<Option<R>> = (0..len).map(|_| None).collect();
    let mut first_failure: Option<(usize, Outcome<R>)> = None;
    for (index, outcome) in outcomes {
        match outcome {
            Outcome::Done(result) => {
                if let Some(slot) = slots.get_mut(index) {
                    *slot = Some(result);
                }
            }
            failure => {
                if first_failure
                    .as_ref()
                    .is_none_or(|(first, _)| index < *first)
                {
                    first_failure = Some((index, failure));
                }
            }
        }
    }
    match first_failure {
        Some((_, Outcome::Failed(err))) => return Err(err),
        Some((_, Outcome::Panicked(payload))) => panic::resume_unwind(payload),
        Some((_, Outcome::Done(_))) | None => {}
    }
    // Without a failure, every chunk was worked on, so every slot is filled.
    slots
        .into_iter()
        .collect::<Option<Vec<R>>>()
        .ok_or_else(|| {
            LinderaErrorKind::Content
                .with_error(anyhow::anyhow!("a text of the batch was not tokenized"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pool and the tokenizer it is made from can be moved to, and
    /// shared with, other threads.
    #[test]
    fn test_pool_is_send_and_tokenizer_is_send_and_sync() {
        /// Compiles only for a `Send` type.
        fn assert_send<T: Send>() {}
        /// Compiles only for a `Send + Sync` type.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send::<AnalysisWorkerPool>();
        assert_send_sync::<Tokenizer>();
    }

    #[cfg(feature = "embed-ipadic")]
    mod with_ipadic {
        use std::collections::HashMap;
        use std::num::NonZeroUsize;
        use std::panic::{self, AssertUnwindSafe};
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::thread;
        use std::time::Duration;

        use lindera_segmenter::LinderaResult;
        use lindera_segmenter::dictionary::{WordId, load_dictionary, load_user_dictionary};
        use lindera_segmenter::error::LinderaErrorKind;
        use lindera_segmenter::mode::{Mode, Penalty};
        use lindera_segmenter::segmenter::Segmenter;
        use lindera_segmenter::token::Token;

        use crate::character_filter::mapping::MappingCharacterFilter;
        use crate::character_filter::{BoxCharacterFilter, CharacterFilter, OffsetMapping};
        use crate::token_filter::BoxTokenFilter;
        use crate::token_filter::lowercase::LowercaseTokenFilter;
        use crate::tokenizer::Tokenizer;
        use crate::worker::AnalysisWorker;
        use crate::worker_pool::AnalysisWorkerPool;

        /// Every field of a token that the comparisons look at.
        type TokenFields = (String, usize, usize, usize, usize, WordId, Vec<String>);

        /// Returns `n` as a `NonZeroUsize`.
        fn threads(n: usize) -> NonZeroUsize {
            match NonZeroUsize::new(n) {
                Some(n) => n,
                None => panic!("a thread count of 0"),
            }
        }

        /// The fields of `tokens`, details included.
        fn fields(tokens: Vec<Token<'_>>) -> Vec<TokenFields> {
            tokens
                .into_iter()
                .map(|mut token| {
                    let details = token.details().iter().map(|d| d.to_string()).collect();
                    (
                        token.surface.to_string(),
                        token.byte_start,
                        token.byte_end,
                        token.position,
                        token.position_length,
                        token.word_id,
                        details,
                    )
                })
                .collect()
        }

        /// IPADIC in Normal mode, with a mapping character filter that
        /// changes byte lengths and a lowercase token filter.
        fn ipadic_tokenizer() -> Tokenizer {
            let dictionary = match load_dictionary("embedded://ipadic") {
                Ok(dictionary) => dictionary,
                Err(err) => panic!("failed to load embedded IPADIC: {err}"),
            };
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let mut tokenizer = Tokenizer::new(segmenter);
            let mut mapping = HashMap::new();
            mapping.insert("リンデラ".to_string(), "Lindera".to_string());
            let filter = match MappingCharacterFilter::new(mapping) {
                Ok(filter) => filter,
                Err(err) => panic!("failed to build mapping filter: {err}"),
            };
            tokenizer.append_character_filter(BoxCharacterFilter::from(filter));
            tokenizer.append_token_filter(BoxTokenFilter::from(LowercaseTokenFilter::new()));
            tokenizer
        }

        /// The non-empty lines of `resources/bocchan.txt`, the longest one
        /// first, so that the first chunk takes the longest.
        fn bocchan_lines() -> Vec<String> {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/bocchan.txt");
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(err) => panic!("failed to read {}: {err}", path.display()),
            };
            let mut lines: Vec<String> = text
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect();
            let longest = (0..lines.len()).max_by_key(|&i| lines[i].len());
            if let Some(longest) = longest {
                let line = lines.remove(longest);
                lines.insert(0, line);
            }
            lines
        }

        /// The fields of every text's tokens, tokenized one after the other
        /// by one worker.
        fn one_by_one<S: AsRef<str>>(
            worker: &mut AnalysisWorker,
            texts: &[S],
        ) -> Vec<Vec<TokenFields>> {
            texts
                .iter()
                .map(|text| match worker.tokenize(text.as_ref()) {
                    Ok(tokens) => fields(tokens),
                    Err(err) => panic!("tokenize failed: {err}"),
                })
                .collect()
        }

        /// The fields of every text's tokens, from one batch on up to
        /// `max_threads` threads, with no lower limit on the input per
        /// thread.
        fn batch<S: AsRef<str> + Sync>(
            pool: &mut AnalysisWorkerPool,
            texts: &[S],
            max_threads: usize,
        ) -> LinderaResult<Vec<Vec<TokenFields>>> {
            pool.run(texts, threads(max_threads), 0, |worker, text| {
                worker.tokenize(text).map(fields)
            })
        }

        /// The results of a batch on 2, 3 and 8 threads are those of one
        /// worker over the same texts, in the same order: the longest text
        /// comes first, so it is still running when the others are done.
        #[test]
        fn test_batch_keeps_input_order() {
            let tokenizer = ipadic_tokenizer();
            let mut texts = bocchan_lines();
            texts.extend(
                ["リンデラは形態素解析器です。", "TOKYO と リンデラ", ""].map(str::to_string),
            );
            let expected = one_by_one(&mut tokenizer.new_worker(), &texts);

            let mut pool = tokenizer.new_worker_pool();
            for max_threads in [2, 3, 8] {
                match batch(&mut pool, &texts, max_threads) {
                    Ok(actual) => assert!(
                        actual == expected,
                        "the batch on {max_threads} threads differs from one worker"
                    ),
                    Err(err) => panic!("the batch failed: {err}"),
                }
            }
        }

        /// The N-best batch gives the results of one worker.
        #[test]
        fn test_nbest_batch_matches_one_worker() {
            let tokenizer = ipadic_tokenizer();
            let texts: Vec<String> = bocchan_lines().into_iter().take(40).collect();
            let mut worker = tokenizer.new_worker();
            let expected: Vec<Vec<(Vec<TokenFields>, i64)>> = texts
                .iter()
                .map(
                    |text| match worker.tokenize_nbest(text, 3, true, Some(5000)) {
                        Ok(results) => results
                            .into_iter()
                            .map(|(tokens, cost)| (fields(tokens), cost))
                            .collect(),
                        Err(err) => panic!("tokenize_nbest failed: {err}"),
                    },
                )
                .collect();

            let mut pool = tokenizer.new_worker_pool();
            let actual = pool.run(&texts, threads(4), 0, |worker, text| {
                worker
                    .tokenize_nbest(text, 3, true, Some(5000))
                    .map(|results| {
                        results
                            .into_iter()
                            .map(|(tokens, cost)| (fields(tokens), cost))
                            .collect::<Vec<_>>()
                    })
            });
            match actual {
                Ok(actual) => assert!(actual == expected, "the N-best batch differs"),
                Err(err) => panic!("the N-best batch failed: {err}"),
            }
        }

        /// A worker's tokenizer tokenizes as the worker does, with the
        /// settings changed by its setters, its filters and its user
        /// dictionary; and so does a pool made from it on several threads.
        #[test]
        fn test_to_tokenizer_keeps_the_workers_configuration() {
            let dictionary = match load_dictionary("embedded://ipadic") {
                Ok(dictionary) => dictionary,
                Err(err) => panic!("failed to load embedded IPADIC: {err}"),
            };
            let csv = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources/user_dict/ipadic_simple_userdic.csv");
            let user_dictionary =
                match load_user_dictionary(&csv.to_string_lossy(), &dictionary.metadata) {
                    Ok(user_dictionary) => user_dictionary,
                    Err(err) => panic!("failed to load the user dictionary: {err}"),
                };
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
            let mut tokenizer = Tokenizer::new(segmenter);
            let mut mapping = HashMap::new();
            mapping.insert("リンデラ".to_string(), "Lindera".to_string());
            let filter = match MappingCharacterFilter::new(mapping) {
                Ok(filter) => filter,
                Err(err) => panic!("failed to build mapping filter: {err}"),
            };
            tokenizer.append_character_filter(BoxCharacterFilter::from(filter));
            tokenizer.append_token_filter(BoxTokenFilter::from(LowercaseTokenFilter::new()));

            let mut worker = tokenizer.new_worker();
            worker.set_mode(Mode::Decompose(Penalty::default()));
            worker.set_keep_whitespace(true);
            worker.set_break_characters("");

            let texts = [
                // The user dictionary has 東京スカイツリー.
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です",
                // Decompose splits a long compound.
                "関西国際空港限定トートバッグ",
                // Kept whitespace.
                " 東京 大阪 ",
                // Without break characters, ・ does not cut.
                "トート・バッグ",
                // The mapping and lowercase filters.
                "リンデラ TOKYO",
            ];
            let expected = one_by_one(&mut worker, &texts);

            // Each setting and part shows in the fixture: the tokenizer the
            // worker was made from, without the setters, differs.
            let unset = one_by_one(&mut tokenizer.new_worker(), &texts);
            for (index, text) in texts.iter().enumerate().skip(1).take(3) {
                assert!(
                    expected[index] != unset[index],
                    "fixture assumption: the setters change {text:?}"
                );
            }

            let mut copy = worker.to_tokenizer().into_worker();
            assert!(
                one_by_one(&mut copy, &texts) == expected,
                "the worker's tokenizer tokenizes differently"
            );

            // Enough texts for every one of 4 threads to take some.
            let many: Vec<&str> = texts.iter().copied().cycle().take(40).collect();
            let expected_many = one_by_one(&mut worker, &many);
            let mut pool = worker.to_tokenizer().into_worker_pool();
            match batch(&mut pool, &many, 4) {
                Ok(actual) => assert!(
                    actual == expected_many,
                    "a pool made from the worker tokenizes differently"
                ),
                Err(err) => panic!("the batch failed: {err}"),
            }
            assert_eq!(pool.worker_count(), 4);
        }

        /// A character filter that waits before a text that starts with
        /// `SLOW`, fails on a text that contains `FAIL`, and panics on a
        /// text that contains `PANIC`.
        #[derive(Clone)]
        struct FaultFilter;

        impl CharacterFilter for FaultFilter {
            /// Returns the filter's name.
            ///
            /// # Returns
            ///
            /// `fault`.
            fn name(&self) -> &str {
                "fault"
            }

            /// Waits, fails or panics as the text says, and leaves it as it
            /// is otherwise.
            ///
            /// # Arguments
            ///
            /// * `text` - The text.
            ///
            /// # Returns
            ///
            /// An empty mapping, or the error for a text with `FAIL`.
            fn apply(&self, text: &mut String) -> LinderaResult<OffsetMapping> {
                if text.starts_with("SLOW") {
                    thread::sleep(Duration::from_millis(100));
                }
                if text.contains("FAIL") {
                    return Err(
                        LinderaErrorKind::Content.with_error(anyhow::anyhow!("failed on {text}"))
                    );
                }
                if text.contains("PANIC") {
                    panic!("panicked on {text}");
                }
                Ok(OffsetMapping::new())
            }
        }

        /// A pool over IPADIC with the fault filter.
        fn fault_pool() -> AnalysisWorkerPool {
            let mut tokenizer = ipadic_tokenizer();
            tokenizer.append_character_filter(BoxCharacterFilter::from(FaultFilter));
            tokenizer.into_worker_pool()
        }

        /// 64 texts, with `early` at index 5 and `late` at index 50.
        fn texts_with(early: &str, late: &str) -> Vec<String> {
            (0..64)
                .map(|index| match index {
                    5 => early.to_string(),
                    50 => late.to_string(),
                    _ => "すもももももももものうち".to_string(),
                })
                .collect()
        }

        /// The pool gives the right tokens after a failed batch.
        fn assert_pool_still_works(pool: &mut AnalysisWorkerPool) {
            let texts = vec!["関西国際空港限定トートバッグ".to_string(); 16];
            let expected = one_by_one(&mut ipadic_tokenizer().into_worker(), &texts);
            match batch(pool, &texts, 4) {
                Ok(actual) => assert!(actual == expected, "the pool tokenizes differently"),
                Err(err) => panic!("the batch after the failure failed: {err}"),
            }
        }

        /// The error of the first failing text in input order is returned,
        /// although a later text fails first.
        #[test]
        fn test_batch_returns_the_first_error_in_input_order() {
            let mut pool = fault_pool();
            let texts = texts_with("SLOW FAIL early", "FAIL late");
            match batch(&mut pool, &texts, 8) {
                Ok(_) => panic!("the batch did not fail"),
                Err(err) => {
                    let message = err.to_string();
                    assert!(message.contains("FAIL early"), "wrong error: {message}");
                }
            }
            assert_pool_still_works(&mut pool);
        }

        /// The panic of the first failing text in input order is resumed,
        /// although a later text panics first, and the pool stays usable.
        #[test]
        fn test_batch_resumes_the_first_panic_in_input_order() {
            let mut pool = fault_pool();
            let texts = texts_with("SLOW PANIC early", "PANIC late");
            let result = panic::catch_unwind(AssertUnwindSafe(|| batch(&mut pool, &texts, 8)));
            match result {
                Ok(_) => panic!("the batch did not panic"),
                Err(payload) => {
                    let message = match payload.downcast_ref::<String>() {
                        Some(message) => message.clone(),
                        None => String::new(),
                    };
                    assert!(message.contains("PANIC early"), "wrong panic: {message:?}");
                }
            }
            assert_pool_still_works(&mut pool);
        }

        /// An error before a panic in input order is returned, and the
        /// panic after it is not resumed.
        #[test]
        fn test_batch_returns_an_error_before_a_panic() {
            let mut pool = fault_pool();
            let texts = texts_with("SLOW FAIL early", "PANIC late");
            let result = panic::catch_unwind(AssertUnwindSafe(|| batch(&mut pool, &texts, 8)));
            match result {
                Ok(Ok(_)) => panic!("the batch did not fail"),
                Ok(Err(err)) => {
                    let message = err.to_string();
                    assert!(message.contains("FAIL early"), "wrong error: {message}");
                }
                Err(_) => panic!("the panic after the error was resumed"),
            }
        }

        /// After a failure, the chunks after it are not worked on: of 1024
        /// texts whose first one fails, far fewer than all are tried.
        #[test]
        fn test_batch_stops_after_a_failure() {
            let mut pool = fault_pool();
            let mut texts = vec!["すもももももももものうち".to_string(); 1024];
            texts[0] = "FAIL first".to_string();
            let tried = AtomicUsize::new(0);
            let result = pool.run(&texts, threads(8), 0, |worker, text| {
                tried.fetch_add(1, Ordering::Relaxed);
                worker.tokenize(text).map(|tokens| tokens.len())
            });
            assert!(result.is_err(), "the batch did not fail");
            let tried = tried.load(Ordering::Relaxed);
            assert!(
                tried < texts.len() / 2,
                "{tried} of {} texts were tried after the first one failed",
                texts.len()
            );
        }

        /// One thread, one text, or a short input runs on the calling
        /// thread alone.
        #[test]
        fn test_small_batches_stay_on_the_calling_thread() {
            let mut pool = ipadic_tokenizer().into_worker_pool();
            let here = thread::current().id();
            let thread_ids =
                |pool: &mut AnalysisWorkerPool, texts: &[String], max_threads: usize| match pool
                    .tokenize_batch(texts, threads(max_threads), |_tokens| {
                        thread::current().id()
                    }) {
                    Ok(ids) => ids,
                    Err(err) => panic!("the batch failed: {err}"),
                };
            let lines = bocchan_lines();

            // One thread.
            let ids = thread_ids(&mut pool, &lines, 1);
            assert!(ids.iter().all(|id| *id == here), "one thread spawned");
            // One text, longer than the input one more thread needs.
            let long = vec![lines.concat()];
            assert!(long[0].len() > 2 * super::super::MIN_BYTES_PER_THREAD);
            let ids = thread_ids(&mut pool, &long, 8);
            assert!(ids.iter().all(|id| *id == here), "one text spawned");
            // Many texts, shorter in all than the input one more thread
            // needs.
            let short = vec!["すもももももももものうち".to_string(); 32];
            assert!(short.concat().len() < 2 * super::super::MIN_BYTES_PER_THREAD);
            let ids = thread_ids(&mut pool, &short, 8);
            assert!(ids.iter().all(|id| *id == here), "a short input spawned");
            assert_eq!(pool.worker_count(), 1, "workers made for no thread");
        }

        /// An empty batch returns no result, and the pool makes no more
        /// workers than a batch runs threads.
        #[test]
        fn test_workers_are_made_as_needed() {
            let mut pool = ipadic_tokenizer().into_worker_pool();
            let none: [&str; 0] = [];
            match batch(&mut pool, &none, 8) {
                Ok(results) => assert!(results.is_empty()),
                Err(err) => panic!("the empty batch failed: {err}"),
            }
            assert_eq!(pool.worker_count(), 1);

            let three = ["東京", "大阪", "京都"];
            match batch(&mut pool, &three, 8) {
                Ok(results) => assert_eq!(results.len(), 3),
                Err(err) => panic!("the batch failed: {err}"),
            }
            assert_eq!(pool.worker_count(), 3);

            pool.truncate_workers(threads(1));
            assert_eq!(pool.worker_count(), 1);
        }
    }
}
