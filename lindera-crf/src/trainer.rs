use core::{num::NonZeroU32, ops::Range};

use alloc::vec::Vec;

use std::sync::Mutex;
use std::thread;

use hashbrown::{HashMap, HashSet, hash_map::RawEntryMut};

use crate::errors::{Result, RucrfError};
use crate::feature::FeatureProvider;
use crate::forward_backward;
use crate::lattice::Lattice;
use crate::model::RawModel;
use crate::optimizers::lbfgs;
use crate::utils::FromU32;

pub struct LatticesLoss<'a> {
    pub lattices: &'a [Lattice],
    provider: &'a FeatureProvider,
    unigram_weight_indices: &'a [Option<NonZeroU32>],
    bigram_weight_indices: &'a [HashMap<u32, u32>],
    n_threads: usize,
    l2_lambda: Option<f64>,
}

impl<'a> LatticesLoss<'a> {
    pub const fn new(
        lattices: &'a [Lattice],
        provider: &'a FeatureProvider,
        unigram_weight_indices: &'a [Option<NonZeroU32>],
        bigram_weight_indices: &'a [HashMap<u32, u32>],
        n_threads: usize,
        l2_lambda: Option<f64>,
    ) -> Self {
        Self {
            lattices,
            provider,
            unigram_weight_indices,
            bigram_weight_indices,
            n_threads,
            l2_lambda,
        }
    }

    pub fn gradient_partial(&self, param: &[f64], range: Range<usize>) -> Vec<f64> {
        let (s, r) = crossbeam_channel::unbounded();
        for lattice in &self.lattices[range] {
            s.send(lattice).unwrap();
        }
        let gradients = Mutex::new(vec![0.0; param.len()]);
        thread::scope(|scope| {
            for _ in 0..self.n_threads {
                scope.spawn(|| {
                    let mut alphas = vec![];
                    let mut betas = vec![];
                    let mut local_gradients = vec![0.0; param.len()];
                    while let Ok(lattice) = r.try_recv() {
                        let z = forward_backward::calculate_alphas_betas(
                            lattice,
                            self.provider,
                            param,
                            self.unigram_weight_indices,
                            self.bigram_weight_indices,
                            &mut alphas,
                            &mut betas,
                        );
                        forward_backward::update_gradient(
                            lattice,
                            self.provider,
                            param,
                            self.unigram_weight_indices,
                            self.bigram_weight_indices,
                            &alphas,
                            &betas,
                            z,
                            &mut local_gradients,
                        );
                    }
                    #[allow(clippy::significant_drop_in_scrutinee)]
                    for (y, x) in gradients.lock().unwrap().iter_mut().zip(local_gradients) {
                        *y += x;
                    }
                });
            }
        });
        let mut gradients = gradients.into_inner().unwrap();

        if let Some(lambda) = self.l2_lambda {
            for (g, p) in gradients.iter_mut().zip(param) {
                *g += lambda * *p;
            }
        }

        gradients
    }

    pub fn cost(&self, param: &[f64]) -> f64 {
        let (s, r) = crossbeam_channel::unbounded();
        for lattice in self.lattices {
            s.send(lattice).unwrap();
        }
        let mut loss_total = thread::scope(|scope| {
            let mut threads = vec![];
            for _ in 0..self.n_threads {
                let t = scope.spawn(|| {
                    let mut alphas = vec![];
                    let mut betas = vec![];
                    let mut loss_total = 0.0;
                    while let Ok(lattice) = r.try_recv() {
                        let z = forward_backward::calculate_alphas_betas(
                            lattice,
                            self.provider,
                            param,
                            self.unigram_weight_indices,
                            self.bigram_weight_indices,
                            &mut alphas,
                            &mut betas,
                        );
                        let loss = forward_backward::calculate_loss(
                            lattice,
                            self.provider,
                            param,
                            self.unigram_weight_indices,
                            self.bigram_weight_indices,
                            z,
                        );
                        loss_total += loss;
                    }
                    loss_total
                });
                threads.push(t);
            }
            let mut loss_total = 0.0;
            for t in threads {
                let loss = t.join().unwrap();
                loss_total += loss;
            }
            loss_total
        });

        if let Some(lambda) = self.l2_lambda {
            let mut norm2 = 0.0;
            for &p in param {
                norm2 += p * p;
            }
            loss_total += lambda * norm2 * 0.5;
        }

        loss_total
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

/// CRF trainer.
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
pub struct Trainer {
    max_iter: u64,
    n_threads: usize,
    regularization: Regularization,
    lambda: f64,
}

impl Trainer {
    /// Creates a new trainer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_iter: 100,
            n_threads: 1,
            regularization: Regularization::L1,
            lambda: 0.1,
        }
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

    /// Sets regularization settings.
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
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn train(&self, lattices: &[Lattice], mut provider: FeatureProvider) -> RawModel {
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

        let weights = lbfgs::optimize(
            lattices,
            &provider,
            &unigram_weight_indices,
            &bigram_weight_indices,
            &weights_init,
            self.regularization,
            self.lambda,
            self.max_iter,
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

        RawModel::new(
            new_weights,
            new_unigram_weight_indices,
            new_bigram_weight_indices,
            provider,
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

        let result = loss_function.gradient_partial(&weights, 0..lattices.len());

        let norm = expected
            .iter()
            .zip(&result)
            .fold(0.0, |acc, (a, b)| acc + (a - b).abs());

        assert!(norm < 1e-12);
    }
}
