use core::num::NonZeroU32;
use core::ops::Range;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::vec::Vec;

use std::thread;

use hashbrown::{HashMap, HashSet, hash_map::RawEntryMut};

use crate::bigram_table::BigramTable;
use crate::errors::{Result, RucrfError};
use crate::feature::FeatureProvider;
use crate::forward_backward::{self, Alpha, Beta, Features};
use crate::lattice::Lattice;
use crate::model::RawModel;
use crate::optimizers::lbfgs;
use crate::utils::FromU32;

/// Upper bound on the number of chunks a lattice slice is split into.
///
/// The chunk boundaries fix the order in which partial results are combined,
/// so this constant is part of the numerical contract of training: changing it
/// perturbs the learned weights by a few ULP. It also caps how many threads a
/// single gradient or loss evaluation can keep busy.
const MAX_CHUNKS: usize = 64;

/// Smallest number of lattices that justifies its own chunk.
///
/// Without this floor a tiny corpus would be split into many nearly-empty
/// chunks and pay one partial buffer and one merge pass per chunk for almost
/// no work.
const MIN_LATTICES_PER_CHUNK: usize = 8;

/// Soft budget for all per-chunk partial gradients of one reduction, in bytes.
///
/// A chunk's partial gradient is a `param.len()`-sized `f64` buffer, so for
/// large models the chunk count is reduced to keep the total scratch memory
/// bounded. Like [`MAX_CHUNKS`], this constant is part of the numerical
/// contract: changing it changes the partition and therefore the summation
/// order.
const PARTIAL_BUDGET_BYTES: usize = 64 << 20;

/// Returns the number of chunks a lattice slice is split into.
///
/// Deliberately *not* a function of the thread count: the chunk count and the
/// chunk boundaries define the summation order of the training objective, and
/// the summation order must not depend on how many workers happen to run.
/// Thread counts above the returned value simply leave surplus workers idle.
///
/// # Arguments
///
/// * `lattice_count` - Number of lattices to reduce over.
/// * `param_len` - Length of the parameter vector, which sizes one partial
///   gradient buffer.
///
/// # Returns
///
/// The chunk count; `0` when there are no lattices.
fn chunk_count(lattice_count: usize, param_len: usize) -> usize {
    if lattice_count == 0 {
        return 0;
    }
    let partial_bytes = param_len.saturating_mul(size_of::<f64>()).max(1);
    let by_memory = (PARTIAL_BUDGET_BYTES / partial_bytes).max(2);
    lattice_count
        .div_ceil(MIN_LATTICES_PER_CHUNK)
        .min(MAX_CHUNKS)
        .min(by_memory)
        .max(1)
}

/// Returns the half-open lattice range covered by chunk `index`.
///
/// Chunk sizes differ by at most one lattice; the first
/// `lattice_count % chunk_count` chunks are the larger ones. Together with
/// [`chunk_count`] this fixes the partition — and therefore the summation
/// order — independently of the thread count.
///
/// # Arguments
///
/// * `lattice_count` - Total number of lattices being partitioned.
/// * `chunk_count` - Number of chunks, as returned by [`chunk_count`].
/// * `index` - Chunk index in `0..chunk_count`.
///
/// # Returns
///
/// The half-open range of lattice indices covered by the chunk; empty when
/// `chunk_count` is `0`.
fn chunk_range(lattice_count: usize, chunk_count: usize, index: usize) -> Range<usize> {
    if chunk_count == 0 {
        return 0..0;
    }
    let base = lattice_count / chunk_count;
    let rem = lattice_count % chunk_count;
    let extra = index.min(rem);
    let start = index * base + extra;
    let end = start + base + usize::from(index < rem);
    start..end
}

/// Adds `src` into `dst` element by element.
///
/// This is the single place where partial gradients are combined, so the
/// sequential and the parallel path provably perform the same additions.
///
/// # Arguments
///
/// * `dst` - Accumulator, updated in place.
/// * `src` - Partial to add; must have the same length as `dst`.
fn add_assign(dst: &mut [f64], src: &[f64]) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, s) in dst.iter_mut().zip(src) {
        *d += *s;
    }
}

/// One chunk's share of an evaluation: its index, its partial loss and its
/// partial gradient.
type ChunkPartial = (usize, f64, Vec<f64>);

/// Sorts chunk partials by chunk index, adds the partial gradients into `acc`
/// and sums the partial losses from `0.0`, both in ascending index order.
///
/// The ascending fold makes the reduction independent of which worker
/// produced which partial and of the order the workers finished in.
///
/// # Arguments
///
/// * `acc` - Gradient accumulator, updated in place.
/// * `partials` - Chunk partials, in any order.
///
/// # Returns
///
/// The index-ordered sum of the partial losses.
fn reduce_indexed_partials(acc: &mut [f64], mut partials: Vec<ChunkPartial>) -> f64 {
    partials.sort_unstable_by_key(|(index, _, _)| *index);
    let mut total = 0.0;
    for (_, loss, gradient) in &partials {
        add_assign(acc, gradient);
        total += loss;
    }
    total
}

/// Joins a scoped worker thread, re-raising a worker panic in the caller.
///
/// # Arguments
///
/// * `handle` - The scoped join handle to wait on.
///
/// # Returns
///
/// The worker's return value.
fn join_or_resume<T>(handle: thread::ScopedJoinHandle<'_, T>) -> T {
    match handle.join() {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Training objective (negative log-likelihood) over a set of lattices.
///
/// [`Self::evaluate`] returns the loss and its gradient from one pass over
/// the lattices. It splits the lattice slice into chunks whose count and
/// boundaries depend on the lattice count and the parameter length alone (see
/// [`chunk_count`]), and reduces the per-chunk partials in ascending chunk
/// order. `n_threads` therefore changes only how long an evaluation takes,
/// never its result.
pub struct LatticesLoss<'a> {
    /// Lattices of the training corpus.
    lattices: &'a [Lattice],
    /// Provider of per-label feature sets.
    provider: &'a FeatureProvider,
    /// Maps unigram feature ids to weight indices.
    unigram_weight_indices: &'a [Option<NonZeroU32>],
    /// The bigram weight indices of every pair of edges in `lattices`.
    bigrams: BigramTable,
    /// Maximum number of worker threads; a pure performance knob.
    n_threads: usize,
    /// L2 regularization strength, when enabled.
    l2_lambda: Option<f64>,
}

impl<'a> LatticesLoss<'a> {
    /// Creates a new loss function over the given lattices.
    ///
    /// Looks up the bigram weight indices of every pair of edges in the
    /// lattices once, so that the evaluations read them from a table (see
    /// [`BigramTable`]).
    ///
    /// # Arguments
    ///
    /// * `lattices` - Lattices of the training corpus.
    /// * `provider` - Provider of per-label feature sets.
    /// * `unigram_weight_indices` - Maps unigram feature ids to weight indices.
    /// * `bigram_weight_indices` - Maps bigram feature id pairs to weight
    ///   indices.
    /// * `n_threads` - Maximum number of worker threads.
    /// * `l2_lambda` - L2 regularization strength, when enabled.
    ///
    /// # Returns
    ///
    /// The loss function.
    pub fn new(
        lattices: &'a [Lattice],
        provider: &'a FeatureProvider,
        unigram_weight_indices: &'a [Option<NonZeroU32>],
        bigram_weight_indices: &[HashMap<u32, u32>],
        n_threads: usize,
        l2_lambda: Option<f64>,
    ) -> Self {
        Self {
            lattices,
            provider,
            unigram_weight_indices,
            bigrams: BigramTable::new(lattices, provider, bigram_weight_indices),
            n_threads,
            l2_lambda,
        }
    }

    /// Accumulates the gradient of one chunk of lattices into `out` and
    /// returns the chunk's loss.
    ///
    /// Each lattice's forward-backward pass serves both its gradient and its
    /// loss. Lattices are processed in ascending slice order, and the loss is
    /// summed from `0.0` in that order, so given an `out` filled with `+0.0`
    /// both results are a pure function of `param` and the chunk's lattices —
    /// independent of which worker runs the chunk and of what that worker ran
    /// before.
    ///
    /// # Arguments
    ///
    /// * `param` - Current parameter vector.
    /// * `range` - Lattice indices covered by this chunk.
    /// * `alphas` - Reusable forward-score scratch, grown as needed.
    /// * `betas` - Reusable backward-score scratch, grown as needed.
    /// * `out` - Chunk partial gradient, updated in place; the caller must
    ///   pass it filled with `+0.0`.
    ///
    /// # Returns
    ///
    /// The chunk's partial loss.
    fn accumulate_chunk(
        &self,
        param: &[f64],
        range: Range<usize>,
        alphas: &mut Vec<Vec<Alpha>>,
        betas: &mut Vec<Vec<Beta>>,
        out: &mut [f64],
    ) -> f64 {
        let features = Features {
            provider: self.provider,
            unigram_weight_indices: self.unigram_weight_indices,
            bigrams: &self.bigrams,
        };
        let mut total = 0.0;
        for index in range {
            let lattice = &self.lattices[index];
            let pairs = self.bigrams.lattice(index);
            let z = forward_backward::calculate_alphas_betas(
                lattice, pairs, &features, param, alphas, betas,
            );
            forward_backward::update_gradient(
                lattice, pairs, &features, param, alphas, betas, z, out,
            );
            total += forward_backward::calculate_loss(lattice, pairs, &features, param, z);
        }
        total
    }

    /// Runs every chunk on up to `n_threads` workers and returns the chunk
    /// partials.
    ///
    /// Workers claim chunk indices from an atomic ticket, so scheduling stays
    /// greedy (a slow chunk never stalls an idle worker) — but each partial is
    /// keyed by its chunk index, which is all the reduction depends on.
    ///
    /// # Arguments
    ///
    /// * `param` - Current parameter vector.
    /// * `chunks` - Chunk count, as returned by [`chunk_count`].
    ///
    /// # Returns
    ///
    /// One partial per chunk, in no particular order.
    fn partials_parallel(&self, param: &[f64], chunks: usize) -> Vec<ChunkPartial> {
        let lattice_count = self.lattices.len();
        // Relaxed suffices: the ticket only hands out chunk indices, and the
        // partials are published to the caller by the scope's implicit join.
        let next_chunk = AtomicUsize::new(0);
        let workers = self.n_threads.min(chunks);

        thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| {
                        let mut alphas = vec![];
                        let mut betas = vec![];
                        let mut mine = Vec::new();
                        loop {
                            let index = next_chunk.fetch_add(1, Ordering::Relaxed);
                            if index >= chunks {
                                break;
                            }
                            let mut partial = vec![0.0; param.len()];
                            let loss = self.accumulate_chunk(
                                param,
                                chunk_range(lattice_count, chunks, index),
                                &mut alphas,
                                &mut betas,
                                &mut partial,
                            );
                            mine.push((index, loss, partial));
                        }
                        mine
                    })
                })
                .collect();
            handles.into_iter().flat_map(join_or_resume).collect()
        })
    }

    /// Computes the training loss and its gradient over every lattice.
    ///
    /// The lattice slice is split into a fixed number of chunks derived from
    /// the lattice count and the parameter length alone (see [`chunk_count`]).
    /// Each chunk's partial gradient is accumulated from a zeroed buffer and
    /// its partial loss from `0.0`, and the partials are added into the
    /// results in ascending chunk order. Neither the partition nor the
    /// reduction order depends on `n_threads`, so for a given build, lattice
    /// slice and `param` this function returns bit-identical values for every
    /// thread count; `n_threads` changes only how long the call takes. At one
    /// thread no channel, lock or thread is created and the chunks run as a
    /// plain loop.
    ///
    /// Do not accumulate two chunks into one buffer, and do not skip the
    /// final add for a single chunk: `0.0 + x` maps `-0.0` to `+0.0`, so both
    /// shortcuts would make the result depend on the partition.
    ///
    /// # Arguments
    ///
    /// * `param` - Current parameter vector.
    ///
    /// # Returns
    ///
    /// The loss and the gradient, the latter with the same length as
    /// `param`, both including the L2 term when configured.
    pub fn evaluate(&self, param: &[f64]) -> (f64, Vec<f64>) {
        let chunks = chunk_count(self.lattices.len(), param.len());
        let mut gradients = vec![0.0; param.len()];
        let mut loss_total = 0.0;

        if self.n_threads <= 1 || chunks <= 1 {
            let mut alphas = vec![];
            let mut betas = vec![];
            let mut partial = vec![0.0; param.len()];
            for index in 0..chunks {
                partial.fill(0.0);
                loss_total += self.accumulate_chunk(
                    param,
                    chunk_range(self.lattices.len(), chunks, index),
                    &mut alphas,
                    &mut betas,
                    &mut partial,
                );
                add_assign(&mut gradients, &partial);
            }
        } else {
            loss_total =
                reduce_indexed_partials(&mut gradients, self.partials_parallel(param, chunks));
        }

        if let Some(lambda) = self.l2_lambda {
            for (g, p) in gradients.iter_mut().zip(param) {
                *g += lambda * *p;
            }
            let mut norm2 = 0.0;
            for &p in param {
                norm2 += p * p;
            }
            loss_total += lambda * norm2 * 0.5;
        }

        (loss_total, gradients)
    }

    /// Computes the training loss over every lattice; the loss half of
    /// [`Self::evaluate`].
    ///
    /// # Arguments
    ///
    /// * `param` - Current parameter vector.
    ///
    /// # Returns
    ///
    /// The loss, including the L2 term when configured.
    #[cfg(test)]
    pub fn cost(&self, param: &[f64]) -> f64 {
        self.evaluate(param).0
    }

    /// Computes the gradient of the training objective over every lattice;
    /// the gradient half of [`Self::evaluate`].
    ///
    /// # Arguments
    ///
    /// * `param` - Current parameter vector.
    ///
    /// # Returns
    ///
    /// The gradient, with the same length as `param`, including the L2 term
    /// when configured.
    #[cfg(test)]
    pub fn gradient_partial(&self, param: &[f64]) -> Vec<f64> {
        self.evaluate(param).1
    }
}

/// L1- or L2- regularization settings
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
#[derive(Copy, Clone, PartialEq)]
pub enum Regularization {
    /// Performs L1-regularization.
    L1,

    /// Performs L2-regularization.
    L2,

    /// Performs Elastic Net regularization (L1 + L2 combination).
    /// The parameter `l1_ratio` controls the mix: 1.0 = pure L1, 0.0 = pure L2.
    /// L1 penalty = lambda * l1_ratio, L2 penalty = lambda * (1 - l1_ratio).
    ElasticNet {
        /// Ratio of L1 vs L2 penalty (0.0 to 1.0).
        l1_ratio: f64,
    },
}

/// The default tolerance of the convergence test, set with [`Trainer::eta`]:
/// `5e-5`, the default `eta` of MeCab's learner.
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
pub const DEFAULT_ETA: f64 = 5e-5;

/// The default regularization coefficient, set with
/// [`Trainer::regularization`]: `1.0`. With [`Regularization::L2`], the
/// default type, the objective is the training loss plus
/// `lambda * ‖w‖² / 2`, which is that of MeCab's learner with its default
/// `-c 1.0`. MeCab adds `‖w‖² / (2C)`, so `lambda` is `1 / C` in general:
/// `-c 0.5` is `lambda` 2.0.
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
pub const DEFAULT_LAMBDA: f64 = 1.0;

/// Why the optimizer stopped.
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StopReason {
    /// The objective converged: its relative change stayed below the
    /// tolerance set with [`Trainer::eta`] for 3 iterations in a row, or its
    /// gradient vanished.
    Converged,
    /// An iteration left the objective unchanged, so the optimizer could not
    /// make progress. The model can be barely trained when this happens
    /// after a few iterations.
    NoProgress,
    /// The cap set with [`Trainer::max_iter`] stopped the optimizer before it
    /// converged.
    MaxIterations,
    /// The line search of the last iteration failed. The weights are the best
    /// ones of the run.
    LineSearchFailed,
}

impl core::fmt::Display for StopReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Converged => "converged",
            Self::NoProgress => "no progress: an iteration left the objective unchanged",
            Self::MaxIterations => "reached the iteration cap before converging",
            Self::LineSearchFailed => "the line search failed",
        })
    }
}

/// Statistics of a training run, which [`Trainer::train_with_stats`] returns
/// with the model.
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TrainingStats {
    /// The number of iterations that ran: the maximum set with
    /// [`Trainer::max_iter`] when it stopped the optimizer, fewer when the
    /// optimizer stopped earlier, for example because it converged.
    pub iterations: u64,
    /// The number of evaluations of the training loss and its gradient, each
    /// one forward-backward pass over every lattice: one per point at which
    /// the optimizer needed them, line-search trials included.
    pub evaluations: u64,
    /// Why the optimizer stopped.
    pub stop_reason: StopReason,
}

/// CRF trainer.
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
pub struct Trainer {
    max_iter: u64,
    n_threads: usize,
    regularization: Regularization,
    lambda: f64,
    eta: f64,
}

impl Trainer {
    /// Creates a new trainer: L2 regularization with [`DEFAULT_LAMBDA`], at
    /// most 100 iterations, the convergence test with [`DEFAULT_ETA`], and
    /// one thread.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_iter: 100,
            n_threads: 1,
            regularization: Regularization::L2,
            lambda: DEFAULT_LAMBDA,
            eta: DEFAULT_ETA,
        }
    }

    /// Sets the tolerance of the convergence test: the optimizer stops when
    /// the relative change of the objective, `|previous - current| /
    /// |previous|`, stays below `eta` for 3 iterations in a row. The default
    /// is [`DEFAULT_ETA`], the default of MeCab's learner; MeCab counts
    /// every evaluation, line-search trials included, where this test counts
    /// iterations. `0.0` disables the test, so that only the iteration cap
    /// and the optimizer's own tests (a vanishing gradient, an unchanged
    /// objective) stop it.
    ///
    /// # Errors
    ///
    /// `eta` must be finite and >= 0.
    pub fn eta(mut self, eta: f64) -> Result<Self> {
        if !(eta.is_finite() && eta >= 0.0) {
            return Err(RucrfError::invalid_argument("eta must be finite and >= 0"));
        }
        self.eta = eta;
        Ok(self)
    }

    /// Sets the maximum number of iterations.
    ///
    /// # Errors
    ///
    /// `max_iter` must be >= 1.
    pub const fn max_iter(mut self, max_iter: u64) -> Result<Self> {
        if max_iter == 0 {
            return Err(RucrfError::invalid_argument("max_iter must be >= 1"));
        }
        self.max_iter = max_iter;
        Ok(self)
    }

    /// Sets regularization settings. The default is
    /// [`Regularization::L2`] with [`DEFAULT_LAMBDA`].
    ///
    /// # Errors
    ///
    /// `lambda` must be >= 0. For `ElasticNet`, `l1_ratio` must be in [0, 1].
    pub fn regularization(mut self, regularization: Regularization, lambda: f64) -> Result<Self> {
        if lambda < 0.0 {
            return Err(RucrfError::invalid_argument("lambda must be >= 0"));
        }
        if let Regularization::ElasticNet { l1_ratio } = regularization
            && !(0.0..=1.0).contains(&l1_ratio)
        {
            return Err(RucrfError::invalid_argument(
                "l1_ratio must be between 0.0 and 1.0",
            ));
        }
        self.regularization = regularization;
        self.lambda = lambda;
        Ok(self)
    }

    /// Sets the number of threads.
    ///
    /// # Errors
    ///
    /// `n_threads` must be >= 1.
    pub const fn n_threads(mut self, n_threads: usize) -> Result<Self> {
        if n_threads == 0 {
            return Err(RucrfError::invalid_argument("n_thread must be >= 1"));
        }
        self.n_threads = n_threads;
        Ok(self)
    }

    #[inline(always)]
    fn update_unigram_feature(
        provider: &FeatureProvider,
        label: NonZeroU32,
        unigram_weight_indices: &mut Vec<Option<NonZeroU32>>,
        weights: &mut Vec<f64>,
    ) {
        if let Some(feature_set) = provider.get_feature_set(label) {
            for &fid in feature_set.unigram() {
                let fid = usize::from_u32(fid.get() - 1);
                if unigram_weight_indices.len() <= fid + 1 {
                    unigram_weight_indices.resize(fid + 1, None);
                }
                if unigram_weight_indices[fid].is_none() {
                    unigram_weight_indices[fid] =
                        Some(NonZeroU32::new(u32::try_from(weights.len()).unwrap() + 1).unwrap());
                    weights.push(0.0);
                }
            }
        }
    }

    #[inline(always)]
    fn update_bigram_feature(
        provider: &FeatureProvider,
        left_label: Option<NonZeroU32>,
        right_label: Option<NonZeroU32>,
        bigram_weight_indices: &mut Vec<HashMap<u32, u32>>,
        weights: &mut Vec<f64>,
    ) {
        match (left_label, right_label) {
            (Some(left_label), Some(right_label)) => {
                if let (Some(left_feature_set), Some(right_feature_set)) = (
                    provider.get_feature_set(left_label),
                    provider.get_feature_set(right_label),
                ) {
                    let left_features = left_feature_set.bigram_left();
                    let right_features = right_feature_set.bigram_right();
                    for (left_fid, right_fid) in left_features.iter().zip(right_features) {
                        if let (Some(left_fid), Some(right_fid)) = (left_fid, right_fid) {
                            let left_fid = usize::try_from(left_fid.get()).unwrap();
                            let right_fid = right_fid.get();
                            if bigram_weight_indices.len() <= left_fid {
                                bigram_weight_indices.resize(left_fid + 1, HashMap::new());
                            }
                            let features = &mut bigram_weight_indices[left_fid];
                            if let RawEntryMut::Vacant(v) =
                                features.raw_entry_mut().from_key(&right_fid)
                            {
                                v.insert(right_fid, u32::try_from(weights.len()).unwrap());
                                weights.push(0.0);
                            }
                        }
                    }
                }
            }
            (Some(left_label), None) => {
                if let Some(feature_set) = provider.get_feature_set(left_label) {
                    for left_fid in feature_set.bigram_left().iter().flatten() {
                        let left_fid = usize::try_from(left_fid.get()).unwrap();
                        if bigram_weight_indices.len() <= left_fid {
                            bigram_weight_indices.resize(left_fid + 1, HashMap::new());
                        }
                        let features = &mut bigram_weight_indices[left_fid];
                        if let RawEntryMut::Vacant(v) = features.raw_entry_mut().from_key(&0) {
                            v.insert(0, u32::try_from(weights.len()).unwrap());
                            weights.push(0.0);
                        }
                    }
                }
            }
            (None, Some(right_label)) => {
                if let Some(feature_set) = provider.get_feature_set(right_label) {
                    for right_fid in feature_set.bigram_right().iter().flatten() {
                        let right_fid = right_fid.get();
                        if bigram_weight_indices.is_empty() {
                            bigram_weight_indices.resize(1, HashMap::new());
                        }
                        let features = &mut bigram_weight_indices[0];
                        if let RawEntryMut::Vacant(v) =
                            features.raw_entry_mut().from_key(&right_fid)
                        {
                            v.insert(right_fid, u32::try_from(weights.len()).unwrap());
                            weights.push(0.0);
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn update_features(
        lattice: &Lattice,
        provider: &FeatureProvider,
        unigram_weight_indices: &mut Vec<Option<NonZeroU32>>,
        bigram_weight_indices: &mut Vec<HashMap<u32, u32>>,
        weights: &mut Vec<f64>,
    ) {
        for (i, node) in lattice.nodes().iter().enumerate() {
            if i == 0 {
                for curr_edge in node.edges() {
                    Self::update_bigram_feature(
                        provider,
                        None,
                        Some(curr_edge.label),
                        bigram_weight_indices,
                        weights,
                    );
                }
            }
            for curr_edge in node.edges() {
                for next_edge in lattice.nodes()[curr_edge.target()].edges() {
                    Self::update_bigram_feature(
                        provider,
                        Some(curr_edge.label),
                        Some(next_edge.label),
                        bigram_weight_indices,
                        weights,
                    );
                }
                if curr_edge.target() == lattice.nodes().len() - 1 {
                    Self::update_bigram_feature(
                        provider,
                        Some(curr_edge.label),
                        None,
                        bigram_weight_indices,
                        weights,
                    );
                }
                Self::update_unigram_feature(
                    provider,
                    curr_edge.label,
                    unigram_weight_indices,
                    weights,
                );
            }
        }
    }

    /// Trains a model from the given dataset.
    ///
    /// The same as [`train_with_stats`](Self::train_with_stats), without the
    /// statistics of the run.
    ///
    /// # Arguments
    ///
    /// * `lattices` - The training lattices.
    /// * `provider` - The feature sets of the labels.
    ///
    /// # Returns
    ///
    /// The trained model.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn train(&self, lattices: &[Lattice], provider: FeatureProvider) -> RawModel {
        self.train_with_stats(lattices, provider).0
    }

    /// Trains a model from the given dataset, and returns it with the
    /// statistics of the run.
    ///
    /// # Arguments
    ///
    /// * `lattices` - The training lattices.
    /// * `provider` - The feature sets of the labels.
    ///
    /// # Returns
    ///
    /// The trained model, and the statistics of the run, such as the number
    /// of iterations that ran.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn train_with_stats(
        &self,
        lattices: &[Lattice],
        mut provider: FeatureProvider,
    ) -> (RawModel, TrainingStats) {
        let mut unigram_weight_indices = vec![];
        let mut bigram_weight_indices = vec![];
        let mut weights_init = vec![];

        for lattice in lattices {
            Self::update_features(
                lattice,
                &provider,
                &mut unigram_weight_indices,
                &mut bigram_weight_indices,
                &mut weights_init,
            );
        }

        let lbfgs::Optimized {
            weights,
            iterations,
            evaluations,
            stop_reason,
        } = lbfgs::optimize(
            lattices,
            &provider,
            &unigram_weight_indices,
            &bigram_weight_indices,
            &weights_init,
            self.regularization,
            self.lambda,
            self.max_iter,
            self.eta,
            self.n_threads,
        );

        // Removes zero weighted features
        let mut weight_id_map = HashMap::new();
        let mut new_weights = vec![];
        for (i, w) in weights.into_iter().enumerate() {
            if w.abs() < f64::EPSILON {
                continue;
            }
            weight_id_map.insert(
                u32::try_from(i).unwrap(),
                u32::try_from(new_weights.len()).unwrap(),
            );
            new_weights.push(w);
        }
        let mut new_unigram_weight_indices = vec![];
        for old_idx in unigram_weight_indices {
            new_unigram_weight_indices.push(old_idx.and_then(|old_idx| {
                weight_id_map
                    .get(&(old_idx.get() - 1))
                    .and_then(|&new_idx| NonZeroU32::new(new_idx + 1))
            }));
        }
        let mut new_bigram_weight_indices = vec![];
        let mut right_id_used = HashSet::new();
        for fids in bigram_weight_indices {
            let mut new_fids = HashMap::new();
            for (k, v) in fids {
                if let Some(&v) = weight_id_map.get(&v) {
                    new_fids.insert(k, v);
                    right_id_used.insert(k);
                }
            }
            new_bigram_weight_indices.push(new_fids);
        }

        for feature_set in &mut provider.feature_sets {
            let mut new_unigram = vec![];
            for &fid in feature_set.unigram() {
                if new_unigram_weight_indices
                    .get(usize::from_u32(fid.get() - 1))
                    .copied()
                    .flatten()
                    .is_some()
                {
                    new_unigram.push(fid);
                }
            }
            feature_set.unigram = new_unigram;
            for fid in &mut feature_set.bigram_left {
                *fid = fid.filter(|fid| {
                    !new_bigram_weight_indices
                        .get(usize::from_u32(fid.get()))
                        .is_none_or(HashMap::is_empty)
                });
            }
            for fid in &mut feature_set.bigram_right {
                *fid = fid.filter(|fid| right_id_used.contains(&fid.get()));
            }
        }

        let model = RawModel::new(
            new_weights,
            new_unigram_weight_indices,
            new_bigram_weight_indices,
            provider,
        );
        (
            model,
            TrainingStats {
                iterations,
                evaluations,
                stop_reason,
            },
        )
    }
}

impl Default for Trainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::cell::Cell;

    use argmin::core::{CostFunction, Gradient};

    use crate::test_utils::{self, hashmap, logsumexp};

    // 0     1     2     3     4     5
    //  /-1-\ /-2-\ /----3----\ /-4-\
    // *     *     *     *     *     *
    //  \----5----/ \-6-/ \-7-/
    // weights:
    // 0->1: 4 (0-1:1 0-2:3)
    // 0->5: 6 (0-2:3 0-2:3)
    // 1->2: 30 (1-4:13 2-3:17)
    // 2->3: 48 (3-2:21 4-3:27)
    // 2->6: 18 (3-4:13 4-1:5)
    // 5->3: 88 (2-2:46 3-3:42)
    // 5->6: 38 (2-4:18 3-1:20)
    // 6->7: 45 (2-3:17 4-4:6)
    // 3->4: 31 (1-2:11 3-1:20)
    // 7->4: 36 (4-2:26 1-1:10)
    // 4->0: 33 (1-0:9 4-0:24)
    // 1: 6
    // 2: 14
    // 3: 8
    // 4: 10
    // 5: 10
    // 6: 10
    // 7: 10
    //
    // 1-2-3-4: 184 *
    // 1-2-6-7-4: 194
    // 5-3-4: 186
    // 5-6-7-4: 176
    //
    // loss = logsumexp(184,194,186,176) - 184
    #[test]
    fn test_loss() {
        let weights = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 46.0,
            17.0, 18.0, 19.0, 20.0, 21.0, 42.0, 13.0, 24.0, 5.0, 26.0, 27.0, 6.0,
        ];
        let provider = test_utils::generate_test_feature_provider();
        let lattices = vec![test_utils::generate_test_lattice()];
        let unigram_weight_indices = vec![
            NonZeroU32::new(2),
            NonZeroU32::new(4),
            NonZeroU32::new(6),
            NonZeroU32::new(8),
        ];
        let bigram_weight_indices = vec![
            hashmap![0 => 28, 1 => 0, 2 => 2, 3 => 4, 4 => 6],
            hashmap![0 => 8, 1 => 9, 2 => 10, 3 => 11, 4 => 12],
            hashmap![0 => 13, 1 => 14, 2 => 15, 3 => 16, 4 => 17],
            hashmap![0 => 18, 1 => 19, 2 => 20, 3 => 21, 4 => 22],
            hashmap![0 => 23, 1 => 24, 2 => 25, 3 => 26, 4 => 27],
        ];
        let loss_function = LatticesLoss::new(
            &lattices,
            &provider,
            &unigram_weight_indices,
            &bigram_weight_indices,
            1,
            None,
        );

        let expected = logsumexp!(184.0, 194.0, 186.0, 176.0) - 184.0;
        let result = loss_function.cost(&weights);

        assert!((expected - result).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gradient() {
        let weights = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 46.0,
            17.0, 18.0, 19.0, 20.0, 21.0, 42.0, 13.0, 24.0, 5.0, 26.0, 27.0, 6.0,
        ];
        let provider = test_utils::generate_test_feature_provider();
        let lattices = vec![test_utils::generate_test_lattice()];
        let unigram_weight_indices = vec![
            NonZeroU32::new(2),
            NonZeroU32::new(4),
            NonZeroU32::new(6),
            NonZeroU32::new(8),
        ];
        let bigram_weight_indices = vec![
            hashmap![0 => 28, 1 => 0, 2 => 2, 3 => 4, 4 => 6],
            hashmap![0 => 8, 1 => 9, 2 => 10, 3 => 11, 4 => 12],
            hashmap![0 => 13, 1 => 14, 2 => 15, 3 => 16, 4 => 17],
            hashmap![0 => 18, 1 => 19, 2 => 20, 3 => 21, 4 => 22],
            hashmap![0 => 23, 1 => 24, 2 => 25, 3 => 26, 4 => 27],
        ];
        let loss_function = LatticesLoss::new(
            &lattices,
            &provider,
            &unigram_weight_indices,
            &bigram_weight_indices,
            1,
            None,
        );

        let z = logsumexp!(184.0, 194.0, 186.0, 176.0);
        let prob1 = (184.0 - z).exp();
        let prob2 = (194.0 - z).exp();
        let prob3 = (186.0 - z).exp();
        let prob4 = (176.0 - z).exp();

        let mut expected = vec![0.0; 29];
        // unigram gradients
        for i in [1, 3, 5, 7, 1, 5, 7, 1] {
            expected[i] -= 1.0;
        }
        for i in [1, 3, 5, 7, 1, 5, 7, 1] {
            expected[i] += prob1;
        }
        for i in [1, 3, 5, 7, 1, 7, 3, 5, 7, 1] {
            expected[i] += prob2;
        }
        for i in [3, 5, 1, 5, 7, 1] {
            expected[i] += prob3;
        }
        for i in [3, 5, 1, 7, 3, 5, 7, 1] {
            expected[i] += prob4;
        }
        // bigram gradients
        for i in [0, 2, 12, 16, 20, 26, 10, 19, 8, 23] {
            expected[i] -= 1.0;
        }
        for i in [0, 2, 12, 16, 20, 26, 10, 19, 8, 23] {
            expected[i] += prob1;
        }
        for i in [0, 2, 12, 16, 22, 24, 16, 27, 25, 9, 8, 23] {
            expected[i] += prob2;
        }
        for i in [2, 2, 15, 21, 10, 19, 8, 23] {
            expected[i] += prob3;
        }
        for i in [2, 2, 17, 19, 16, 27, 25, 9, 8, 23] {
            expected[i] += prob4;
        }

        let result = loss_function.gradient_partial(&weights);

        let norm = expected
            .iter()
            .zip(&result)
            .fold(0.0, |acc, (a, b)| acc + (a - b).abs());

        assert!(norm < 1e-12);
    }

    fn wide_fixture_tables() -> (Vec<Option<NonZeroU32>>, Vec<HashMap<u32, u32>>) {
        let unigram_weight_indices = vec![
            NonZeroU32::new(2),
            NonZeroU32::new(4),
            NonZeroU32::new(6),
            NonZeroU32::new(8),
        ];
        let bigram_weight_indices = vec![
            hashmap![0 => 28, 1 => 0, 2 => 2, 3 => 4, 4 => 6],
            hashmap![0 => 8, 1 => 9, 2 => 10, 3 => 11, 4 => 12],
            hashmap![0 => 13, 1 => 14, 2 => 15, 3 => 16, 4 => 17],
            hashmap![0 => 18, 1 => 19, 2 => 20, 3 => 21, 4 => 22],
            hashmap![0 => 23, 1 => 24, 2 => 25, 3 => 26, 4 => 27],
        ];
        (unigram_weight_indices, bigram_weight_indices)
    }

    // Full-mantissa weights so that any change of summation order shows in
    // the low bits of the result.
    fn wide_fixture_weights() -> Vec<f64> {
        (0..29)
            .map(|i| ((i + 1) as f64).sqrt() * 0.37 - 0.11)
            .collect()
    }

    #[test]
    fn chunk_range_partitions_the_slice() {
        for lattice_count in 0..40 {
            for chunks in 1..=8 {
                let mut next_start = 0;
                let mut min_size = usize::MAX;
                let mut max_size = 0;
                for index in 0..chunks {
                    let range = chunk_range(lattice_count, chunks, index);
                    assert_eq!(
                        range.start, next_start,
                        "chunk {index} of {chunks} over {lattice_count} is not contiguous"
                    );
                    let size = range.end - range.start;
                    min_size = min_size.min(size);
                    max_size = max_size.max(size);
                    next_start = range.end;
                }
                assert_eq!(
                    next_start, lattice_count,
                    "{chunks} chunks over {lattice_count} lattices do not cover the slice"
                );
                assert!(
                    max_size - min_size <= 1,
                    "chunk sizes over {lattice_count} lattices differ by more than one"
                );
            }
        }
    }

    // The specification test: the full gradient must equal the ascending-
    // chunk-order fold of each chunk computed on its own. This pins the
    // reduction *structure*, not just the agreement of two runs, so it fails
    // on the pre-fix code whose one-thread result is a flat sum.
    #[test]
    fn gradient_equals_chunkwise_reduction() {
        let weights = wide_fixture_weights();
        let provider = test_utils::generate_test_feature_provider();
        let lattices = test_utils::generate_test_lattices(200);
        let (uni, bi) = wide_fixture_tables();

        let chunks = chunk_count(lattices.len(), weights.len());
        assert!(chunks > 1, "fixture too small to span several chunks");

        let full =
            LatticesLoss::new(&lattices, &provider, &uni, &bi, 1, None).gradient_partial(&weights);

        let mut folded = vec![0.0; weights.len()];
        for index in 0..chunks {
            let range = chunk_range(lattices.len(), chunks, index);
            let sub = &lattices[range];
            // A chunk holds at most MIN_LATTICES_PER_CHUNK lattices here, so
            // evaluating it alone must not split it further -- otherwise this
            // would not be a raw chunk partial.
            assert_eq!(chunk_count(sub.len(), weights.len()), 1);
            let partial =
                LatticesLoss::new(sub, &provider, &uni, &bi, 1, None).gradient_partial(&weights);
            add_assign(&mut folded, &partial);
        }

        for (k, (a, b)) in full.iter().zip(&folded).enumerate() {
            assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "component {k}: full {a} != chunkwise {b}"
            );
        }
    }

    #[test]
    fn gradient_is_bit_identical_across_thread_counts() {
        let weights = wide_fixture_weights();
        let provider = test_utils::generate_test_feature_provider();
        let lattices = test_utils::generate_test_lattices(200);
        let (uni, bi) = wide_fixture_tables();

        assert!(
            chunk_count(lattices.len(), weights.len()) > 1,
            "fixture too small to span several chunks"
        );

        let baseline =
            LatticesLoss::new(&lattices, &provider, &uni, &bi, 1, None).gradient_partial(&weights);
        assert!(
            baseline.iter().all(|g| g.is_finite()),
            "fixture produced a non-finite gradient"
        );
        assert!(
            baseline.iter().any(|g| *g != 0.0),
            "vacuous fixture: all-zero gradient"
        );

        for n_threads in [2, 3, 4, 8, 16] {
            let result = LatticesLoss::new(&lattices, &provider, &uni, &bi, n_threads, None)
                .gradient_partial(&weights);
            assert_eq!(result.len(), baseline.len());
            for (k, (a, b)) in baseline.iter().zip(&result).enumerate() {
                assert_eq!(
                    a.to_bits(),
                    b.to_bits(),
                    "component {k} differs at n_threads={n_threads}: {a} vs {b}"
                );
            }
        }
    }

    #[test]
    fn cost_is_bit_identical_across_thread_counts() {
        let weights = wide_fixture_weights();
        let provider = test_utils::generate_test_feature_provider();
        let lattices = test_utils::generate_test_lattices(200);
        let (uni, bi) = wide_fixture_tables();

        assert!(
            chunk_count(lattices.len(), weights.len()) > 1,
            "fixture too small to span several chunks"
        );

        for l2_lambda in [None, Some(0.01)] {
            let baseline =
                LatticesLoss::new(&lattices, &provider, &uni, &bi, 1, l2_lambda).cost(&weights);
            assert!(baseline.is_finite() && baseline != 0.0, "vacuous fixture");

            for n_threads in [2, 3, 4, 8, 16] {
                let result =
                    LatticesLoss::new(&lattices, &provider, &uni, &bi, n_threads, l2_lambda)
                        .cost(&weights);
                assert_eq!(
                    baseline.to_bits(),
                    result.to_bits(),
                    "cost differs at n_threads={n_threads} (l2={l2_lambda:?}): \
                     {baseline} vs {result}"
                );
            }
        }
    }

    // Issue #980's repro, in unit form: repeated evaluations at a high thread
    // count must all agree bit for bit.
    #[test]
    fn gradient_is_stable_across_repeated_runs() {
        let weights = wide_fixture_weights();
        let provider = test_utils::generate_test_feature_provider();
        let lattices = test_utils::generate_test_lattices(200);
        let (uni, bi) = wide_fixture_tables();

        let loss = LatticesLoss::new(&lattices, &provider, &uni, &bi, 8, None);
        let first = loss.gradient_partial(&weights);
        for run in 1..20 {
            let again = loss.gradient_partial(&weights);
            for (k, (a, b)) in first.iter().zip(&again).enumerate() {
                assert_eq!(
                    a.to_bits(),
                    b.to_bits(),
                    "component {k} changed on run {run}: {a} vs {b}"
                );
            }
        }
    }

    #[test]
    fn more_threads_than_chunks_matches_single_thread() {
        let weights = wide_fixture_weights();
        let provider = test_utils::generate_test_feature_provider();
        let lattices = test_utils::generate_test_lattices(24);
        let (uni, bi) = wide_fixture_tables();

        // Derive the thread count from the data so the test cannot silently
        // stop exercising the surplus-worker path.
        let chunks = chunk_count(lattices.len(), weights.len());
        assert!(chunks >= 2, "fixture too small to span several chunks");
        let n_threads = chunks + 3;

        let baseline =
            LatticesLoss::new(&lattices, &provider, &uni, &bi, 1, None).gradient_partial(&weights);
        let surplus = LatticesLoss::new(&lattices, &provider, &uni, &bi, n_threads, None)
            .gradient_partial(&weights);
        for (k, (a, b)) in baseline.iter().zip(&surplus).enumerate() {
            assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "component {k} differs with {n_threads} threads over {chunks} chunks"
            );
        }
    }

    #[test]
    fn empty_lattices_yield_zero_gradient_and_cost() {
        let weights = wide_fixture_weights();
        let provider = test_utils::generate_test_feature_provider();
        let lattices: Vec<Lattice> = vec![];
        let (uni, bi) = wide_fixture_tables();

        for n_threads in [1, 8] {
            let loss = LatticesLoss::new(&lattices, &provider, &uni, &bi, n_threads, None);
            let gradient = loss.gradient_partial(&weights);
            // argmin's vector arithmetic depends on the length, so it must be
            // param-sized even with nothing to sum.
            assert_eq!(gradient.len(), weights.len());
            assert!(gradient.iter().all(|g| *g == 0.0));
            assert_eq!(loss.cost(&weights), 0.0);

            let lambda = 0.05;
            let loss = LatticesLoss::new(&lattices, &provider, &uni, &bi, n_threads, Some(lambda));
            let gradient = loss.gradient_partial(&weights);
            assert_eq!(gradient.len(), weights.len());
            for (k, (g, p)) in gradient.iter().zip(&weights).enumerate() {
                assert_eq!(
                    g.to_bits(),
                    (lambda * *p).to_bits(),
                    "L2 gradient component {k}"
                );
            }
            let mut norm2 = 0.0;
            for &p in &weights {
                norm2 += p * p;
            }
            assert_eq!(
                loss.cost(&weights).to_bits(),
                (lambda * norm2 * 0.5).to_bits()
            );
        }
    }

    // Fails by construction under any reordering, not probabilistically:
    // with ties-to-even, (1.0 + 2^-53) + 2^-53 == 1.0, while summing the two
    // 2^-53 terms first gives 2^-52, and 1.0 + 2^-52 is the next float up.
    // So only the ascending-index fold produces exactly 1.0 from these
    // shuffled partials.
    #[test]
    fn partials_are_reduced_in_index_order() {
        let eps = (2.0f64).powi(-53);

        let mut acc = vec![0.0f64];
        let scalar = reduce_indexed_partials(
            &mut acc,
            vec![
                (1, eps, vec![eps]),
                (2, eps, vec![eps]),
                (0, 1.0, vec![1.0]),
            ],
        );
        assert_eq!(scalar.to_bits(), 1.0f64.to_bits(), "scalar fold: {scalar}");
        assert_eq!(
            acc[0].to_bits(),
            1.0f64.to_bits(),
            "vector fold: {}",
            acc[0]
        );
    }

    // The statistics count the iterations that ran, not the cap (#1169).
    #[test]
    fn train_with_stats_reports_the_iterations_that_ran() {
        let lattices = test_utils::generate_test_lattices(24);
        let iterations = |max_iter| {
            let trainer = Trainer::new().max_iter(max_iter).unwrap();
            let (_, stats) =
                trainer.train_with_stats(&lattices, test_utils::generate_test_feature_provider());
            stats.iterations
        };

        // The optimizer stops before a high cap...
        let high = iterations(10_000);
        assert!((1..10_000).contains(&high), "{high} iterations");
        // ...and at a low one.
        assert_eq!(iterations(1), 1);
    }

    // The convergence test (#1167) stops training before the cap once the
    // relative change of the objective has stayed below eta for 3
    // iterations, and eta = 0 turns it off.
    #[test]
    fn eta_stops_training_once_the_objective_converges() {
        let lattices = test_utils::generate_test_lattices(24);
        let run = |eta, max_iter| {
            let trainer = Trainer::new().max_iter(max_iter).unwrap().eta(eta).unwrap();
            trainer
                .train_with_stats(&lattices, test_utils::generate_test_feature_provider())
                .1
        };

        // A loose tolerance passes from the first iteration on, so the third
        // check after the initial one stops the run.
        let loose = run(1.0, 1000);
        assert_eq!(loose.stop_reason, StopReason::Converged);
        assert_eq!(loose.iterations, 3);

        // Without the test, a cap below where argmin's own tests stop this
        // fixture is what stops it.
        let off = run(0.0, 5);
        assert_eq!(off.stop_reason, StopReason::MaxIterations);
        assert_eq!(off.iterations, 5);
    }

    #[test]
    fn eta_must_be_finite_and_not_negative() {
        for eta in [-1e-9, f64::NAN, f64::INFINITY] {
            assert!(Trainer::new().eta(eta).is_err(), "eta {eta}");
        }
        for eta in [0.0, DEFAULT_ETA, 1.0] {
            assert!(Trainer::new().eta(eta).is_ok(), "eta {eta}");
        }
    }

    // A new trainer regularizes with L2 and DEFAULT_LAMBDA, 1.0, the
    // objective of MeCab's learner with its default -c 1.0 (#1261).
    #[test]
    fn new_defaults_to_l2_with_the_default_lambda() {
        let trainer = Trainer::new();
        assert!(trainer.regularization == Regularization::L2);
        assert_eq!(trainer.lambda.to_bits(), DEFAULT_LAMBDA.to_bits());
        assert_eq!(DEFAULT_LAMBDA.to_bits(), 1.0f64.to_bits());
    }

    // The L1 run of the wide fixture from zero weights reaches an iteration
    // that leaves the cost unchanged, as the run of #1260 did; the reason
    // says so instead of claiming convergence.
    #[test]
    fn an_unchanged_objective_is_reported_as_no_progress() {
        let provider = test_utils::generate_test_feature_provider();
        let lattices = test_utils::generate_test_lattices(200);
        let (uni, bi) = wide_fixture_tables();
        let evaluations = Cell::new(0);
        let result = lbfgs::minimize(
            lbfgs::CachedLoss::new(
                LatticesLoss::new(&lattices, &provider, &uni, &bi, 1, None),
                &evaluations,
            ),
            vec![0.0; 29],
            Regularization::L1,
            0.01,
            100,
            0.0,
        );
        assert_eq!(result.stop_reason, StopReason::NoProgress);
        assert!(result.iterations < 100, "{} iterations", result.iterations);
    }

    // The evaluation as `cost` and `gradient_partial` computed it before they
    // shared one pass (#1167): a forward-backward pass of its own for the
    // loss and for the gradient, over the same chunks, each folded in
    // ascending chunk order.
    fn reference_evaluation(
        lattices: &[Lattice],
        provider: &FeatureProvider,
        uni: &[Option<NonZeroU32>],
        bi: &[HashMap<u32, u32>],
        l2_lambda: Option<f64>,
        param: &[f64],
    ) -> (f64, Vec<f64>) {
        let chunks = chunk_count(lattices.len(), param.len());
        let mut alphas = vec![];
        let mut betas = vec![];

        let mut loss_total = 0.0;
        for index in 0..chunks {
            let mut total = 0.0;
            for lattice in &lattices[chunk_range(lattices.len(), chunks, index)] {
                let z = forward_backward::reference::calculate_alphas_betas(
                    lattice,
                    provider,
                    param,
                    uni,
                    bi,
                    &mut alphas,
                    &mut betas,
                );
                total += forward_backward::reference::calculate_loss(
                    lattice, provider, param, uni, bi, z,
                );
            }
            loss_total += total;
        }

        let mut gradients = vec![0.0; param.len()];
        let mut partial = vec![0.0; param.len()];
        for index in 0..chunks {
            partial.fill(0.0);
            for lattice in &lattices[chunk_range(lattices.len(), chunks, index)] {
                let z = forward_backward::reference::calculate_alphas_betas(
                    lattice,
                    provider,
                    param,
                    uni,
                    bi,
                    &mut alphas,
                    &mut betas,
                );
                forward_backward::reference::update_gradient(
                    lattice,
                    provider,
                    param,
                    uni,
                    bi,
                    &alphas,
                    &betas,
                    z,
                    &mut partial,
                );
            }
            add_assign(&mut gradients, &partial);
        }

        if let Some(lambda) = l2_lambda {
            for (g, p) in gradients.iter_mut().zip(param) {
                *g += lambda * *p;
            }
            let mut norm2 = 0.0;
            for &p in param {
                norm2 += p * p;
            }
            loss_total += lambda * norm2 * 0.5;
        }
        (loss_total, gradients)
    }

    /// Parameter vectors with full-mantissa components of both signs, so
    /// that a change of summation order shows in the low bits.
    fn evaluation_params() -> Vec<Vec<f64>> {
        let wide = wide_fixture_weights();
        let flipped = wide
            .iter()
            .enumerate()
            .map(|(i, w)| if i % 3 == 0 { -w * 1.7 } else { w * 0.3 })
            .collect();
        vec![wide, flipped, vec![0.0; 29]]
    }

    #[test]
    fn evaluation_matches_the_reference_bit_for_bit() {
        let (uni, bi) = wide_fixture_tables();
        // The regular lattices, and lattices with parallel edges, a dead end,
        // a node without incoming edges, empty bigram slots and a label
        // without a feature set.
        let fixtures = [
            (
                "regular",
                test_utils::generate_test_feature_provider(),
                test_utils::generate_test_lattices(200),
            ),
            (
                "irregular",
                test_utils::generate_irregular_feature_provider(),
                test_utils::generate_irregular_lattices(60),
            ),
        ];
        for (fixture, provider, lattices) in &fixtures {
            assert!(
                chunk_count(lattices.len(), 29) > 1,
                "{fixture}: fixture too small to span several chunks"
            );
            evaluation_matches_the_reference(fixture, provider, lattices, &uni, &bi);
        }
    }

    fn evaluation_matches_the_reference(
        fixture: &str,
        provider: &FeatureProvider,
        lattices: &[Lattice],
        uni: &[Option<NonZeroU32>],
        bi: &[HashMap<u32, u32>],
    ) {
        for param in evaluation_params() {
            for l2_lambda in [None, Some(0.01)] {
                let (cost, gradient) =
                    reference_evaluation(lattices, provider, uni, bi, l2_lambda, &param);
                for n_threads in [1, 2, 3, 8] {
                    let (got_cost, got_gradient) =
                        LatticesLoss::new(lattices, provider, uni, bi, n_threads, l2_lambda)
                            .evaluate(&param);
                    assert_eq!(
                        got_cost.to_bits(),
                        cost.to_bits(),
                        "{fixture}: cost at n_threads={n_threads}, l2={l2_lambda:?}: \
                         {got_cost} vs {cost}"
                    );
                    for (k, (a, b)) in got_gradient.iter().zip(&gradient).enumerate() {
                        assert_eq!(
                            a.to_bits(),
                            b.to_bits(),
                            "{fixture}: gradient component {k} at n_threads={n_threads}, \
                             l2={l2_lambda:?}: {a} vs {b}"
                        );
                    }
                }
            }
        }
    }

    /// The objective without a cache: every call runs its own evaluation, as
    /// the trait implementations on `LatticesLoss` did before `CachedLoss`.
    struct Uncached<'a>(LatticesLoss<'a>);

    impl CostFunction for Uncached<'_> {
        type Param = Vec<f64>;
        type Output = f64;

        fn cost(&self, param: &Self::Param) -> core::result::Result<f64, argmin::core::Error> {
            Ok(self.0.cost(param))
        }
    }

    impl Gradient for Uncached<'_> {
        type Param = Vec<f64>;
        type Gradient = Vec<f64>;

        fn gradient(
            &self,
            param: &Self::Param,
        ) -> core::result::Result<Vec<f64>, argmin::core::Error> {
            Ok(self.0.gradient_partial(param))
        }
    }

    /// Counts the cost and gradient calls argmin makes on `inner`, and the
    /// gradient calls that made `inner` compute an evaluation.
    struct Counting<'a, P> {
        inner: P,
        evaluations: &'a Cell<u64>,
        gradients: &'a Cell<u64>,
        gradient_misses: &'a Cell<u64>,
    }

    impl<P: CostFunction<Param = Vec<f64>, Output = f64>> CostFunction for Counting<'_, P> {
        type Param = Vec<f64>;
        type Output = f64;

        fn cost(&self, param: &Self::Param) -> core::result::Result<f64, argmin::core::Error> {
            self.inner.cost(param)
        }
    }

    impl<P: Gradient<Param = Vec<f64>, Gradient = Vec<f64>>> Gradient for Counting<'_, P> {
        type Param = Vec<f64>;
        type Gradient = Vec<f64>;

        fn gradient(
            &self,
            param: &Self::Param,
        ) -> core::result::Result<Vec<f64>, argmin::core::Error> {
            self.gradients.set(self.gradients.get() + 1);
            let before = self.evaluations.get();
            let gradient = self.inner.gradient(param);
            if self.evaluations.get() != before {
                self.gradient_misses.set(self.gradient_misses.get() + 1);
            }
            gradient
        }
    }

    // Training through argmin with the cached objective must take exactly
    // the steps of the uncached one, and compute one evaluation per point:
    // every gradient argmin asks for is at the point of the preceding cost.
    #[test]
    fn cached_training_matches_uncached_training() {
        let provider = test_utils::generate_test_feature_provider();
        let lattices = test_utils::generate_test_lattices(200);
        let (uni, bi) = wide_fixture_tables();
        let lambda = 0.01;
        let max_iter = 20;

        for (name, regularization) in [
            ("L1", Regularization::L1),
            ("L2", Regularization::L2),
            ("Elastic Net", Regularization::ElasticNet { l1_ratio: 0.5 }),
        ] {
            let l2_lambda = lbfgs::l2_lambda(regularization, lambda);
            // eta = 0 leaves the cap to stop both runs.
            let lbfgs::Minimized {
                weights: expected,
                iterations: expected_iterations,
                ..
            } = lbfgs::minimize(
                Uncached(LatticesLoss::new(
                    &lattices, &provider, &uni, &bi, 1, l2_lambda,
                )),
                wide_fixture_weights(),
                regularization,
                lambda,
                max_iter,
                0.0,
            );

            let evaluations = Cell::new(0);
            let gradients = Cell::new(0);
            let gradient_misses = Cell::new(0);
            let cached = lbfgs::CachedLoss::new(
                LatticesLoss::new(&lattices, &provider, &uni, &bi, 1, l2_lambda),
                &evaluations,
            );
            let lbfgs::Minimized {
                weights,
                iterations,
                ..
            } = lbfgs::minimize(
                Counting {
                    inner: cached,
                    evaluations: &evaluations,
                    gradients: &gradients,
                    gradient_misses: &gradient_misses,
                },
                wide_fixture_weights(),
                regularization,
                lambda,
                max_iter,
                0.0,
            );

            assert_eq!(iterations, expected_iterations, "{name}");
            assert!(iterations >= 5, "{name}: only {iterations} iterations ran");
            for (k, (a, b)) in weights.iter().zip(&expected).enumerate() {
                assert_eq!(a.to_bits(), b.to_bits(), "{name}: weight {k}: {a} vs {b}");
            }
            assert!(gradients.get() > 0, "{name}: no gradient call");
            assert_eq!(
                gradient_misses.get(),
                0,
                "{name}: {} of {} gradient calls computed an evaluation",
                gradient_misses.get(),
                gradients.get()
            );
        }
    }
}
