use core::num::NonZeroU32;

use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::feature::{self, FeatureProvider};
use crate::lattice::Lattice;
use crate::math;
use crate::utils::FromU32;

#[derive(Copy, Clone)]
pub struct Alpha {
    pub prev_node: usize,
    pub label: Option<NonZeroU32>,
    pub score: f64,
}

impl Alpha {
    const fn new(prev_node: usize, label: Option<NonZeroU32>, score: f64) -> Self {
        Self {
            prev_node,
            label,
            score,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Beta {
    pub next_node: usize,
    pub label: Option<NonZeroU32>,
    pub score: f64,
}

impl Beta {
    const fn new(next_node: usize, label: Option<NonZeroU32>, score: f64) -> Self {
        Self {
            next_node,
            label,
            score,
        }
    }
}

pub fn calculate_alphas_betas(
    lattice: &Lattice,
    provider: &FeatureProvider,
    weights: &[f64],
    unigram_weight_indices: &[Option<NonZeroU32>],
    bigram_weight_indices: &[HashMap<u32, u32>],
    alphas: &mut Vec<Vec<Alpha>>,
    betas: &mut Vec<Vec<Beta>>,
) -> f64 {
    if alphas.len() < lattice.nodes().len() {
        alphas.resize(lattice.nodes().len(), vec![]);
        betas.resize(lattice.nodes().len(), vec![]);
    }
    for x in &mut alphas[..lattice.nodes().len()] {
        x.clear();
    }
    for x in &mut betas[..lattice.nodes().len()] {
        x.clear();
    }
    alphas[0].push(Alpha::new(0, None, 0.0));
    betas[lattice.nodes().len() - 1].push(Beta::new(0, None, 0.0));

    // add 1-gram scores
    for (i, (node, betas)) in lattice.nodes().iter().zip(betas.iter_mut()).enumerate() {
        for edge in node.edges() {
            let mut score = 0.0;
            if let Some(feature_set) = provider.get_feature_set(edge.label) {
                for &fid in feature_set.unigram() {
                    let fid = usize::try_from(fid.get() - 1).unwrap();
                    let widx = unigram_weight_indices[fid].unwrap().get() - 1;
                    score += weights[usize::from_u32(widx)];
                }
            }
            alphas[edge.target()].push(Alpha::new(i, Some(edge.label), score));
            betas.push(Beta::new(edge.target(), Some(edge.label), score));
        }
    }

    // alphas
    for i in 1..lattice.nodes().len() {
        for j in 0..alphas[i].len() {
            let Alpha {
                prev_node,
                label: curr_label,
                ..
            } = alphas[i][j];
            let mut score_total = f64::NEG_INFINITY;
            for &Alpha {
                label: prev_label,
                mut score,
                ..
            } in &alphas[prev_node]
            {
                feature::apply_bigram(
                    prev_label,
                    curr_label,
                    provider,
                    bigram_weight_indices,
                    |widx| {
                        score += weights[usize::from_u32(widx)];
                    },
                );
                score_total = math::logsumexp(score_total, score);
            }
            alphas[i][j].score += score_total;
        }
    }

    // betas
    for i in (0..lattice.nodes().len() - 1).rev() {
        for j in 0..betas[i].len() {
            let Beta {
                next_node,
                label: curr_label,
                ..
            } = betas[i][j];
            let mut score_total = f64::NEG_INFINITY;
            for &Beta {
                label: next_label,
                mut score,
                ..
            } in &betas[next_node]
            {
                feature::apply_bigram(
                    curr_label,
                    next_label,
                    provider,
                    bigram_weight_indices,
                    |widx| {
                        score += weights[usize::from_u32(widx)];
                    },
                );
                score_total = math::logsumexp(score_total, score);
            }
            betas[i][j].score += score_total;
        }
    }

    let mut score_total = f64::NEG_INFINITY;
    for &Beta {
        label: next_label,
        mut score,
        ..
    } in &betas[0]
    {
        feature::apply_bigram(None, next_label, provider, bigram_weight_indices, |widx| {
            score += weights[usize::from_u32(widx)];
        });
        score_total = math::logsumexp(score_total, score);
    }
    score_total
}

pub fn calculate_loss(
    lattice: &Lattice,
    provider: &FeatureProvider,
    weights: &[f64],
    unigram_weight_indices: &[Option<NonZeroU32>],
    bigram_weight_indices: &[HashMap<u32, u32>],
    z: f64,
) -> f64 {
    let mut log_prob = z;
    let mut pos = 0;
    let mut prev_label = None;

    while pos < lattice.nodes().len() - 1 {
        let edge = &lattice.nodes()[pos].edges()[0];
        if let Some(feature_set) = provider.get_feature_set(edge.label) {
            for &fid in feature_set.unigram() {
                let fid = usize::from_u32(fid.get() - 1);
                let widx = unigram_weight_indices[fid].unwrap().get() - 1;
                log_prob -= weights[usize::from_u32(widx)];
            }
        }
        feature::apply_bigram(
            prev_label,
            Some(edge.label),
            provider,
            bigram_weight_indices,
            |widx| {
                log_prob -= weights[usize::from_u32(widx)];
            },
        );
        pos = edge.target();
        prev_label = Some(edge.label);
    }
    if let Some(feature_set) = provider.get_feature_set(prev_label.unwrap()) {
        for &prev_fid in feature_set.bigram_left() {
            if let Some(prev_fid) = prev_fid {
                let prev_fid = usize::try_from(prev_fid.get()).unwrap();
                if let Some(&widx) = bigram_weight_indices[prev_fid].get(&0) {
                    log_prob -= weights[usize::from_u32(widx)];
                }
            }
        }
    }
    log_prob
}

#[allow(clippy::too_many_arguments)]
pub fn update_gradient(
    lattice: &Lattice,
    provider: &FeatureProvider,
    weights: &[f64],
    unigram_weight_indices: &[Option<NonZeroU32>],
    bigram_weight_indices: &[HashMap<u32, u32>],
    alphas: &[Vec<Alpha>],
    betas: &[Vec<Beta>],
    z: f64,
    gradients: &mut [f64],
) {
    for (alphas, betas) in alphas.iter().zip(betas).take(lattice.nodes().len()) {
        for alpha in alphas {
            let mut prob_total = 0.0;
            for beta in betas {
                let mut log_prob = alpha.score + beta.score - z;
                feature::apply_bigram(
                    alpha.label,
                    beta.label,
                    provider,
                    bigram_weight_indices,
                    |widx| {
                        log_prob += weights[usize::from_u32(widx)];
                    },
                );
                let prob = log_prob.exp();
                prob_total += prob;
                feature::apply_bigram(
                    alpha.label,
                    beta.label,
                    provider,
                    bigram_weight_indices,
                    |widx| {
                        gradients[usize::from_u32(widx)] += prob;
                    },
                );
            }
            if let Some(prev_label) = alpha.label {
                if let Some(feature_set) = provider.get_feature_set(prev_label) {
                    for &fid in feature_set.unigram() {
                        let fid = usize::try_from(fid.get() - 1).unwrap();
                        let widx = unigram_weight_indices[fid].unwrap().get() - 1;
                        gradients[usize::from_u32(widx)] += prob_total;
                    }
                }
            }
        }
    }
    let mut pos = 0;
    let mut prev_label = None;
    while pos < lattice.nodes().len() - 1 {
        let edge = &lattice.nodes()[pos].edges()[0];
        if let Some(feature_set) = provider.get_feature_set(edge.label) {
            for &fid in feature_set.unigram() {
                let fid = usize::try_from(fid.get() - 1).unwrap();
                let widx = unigram_weight_indices[fid].unwrap().get() - 1;
                gradients[usize::from_u32(widx)] -= 1.0;
            }
        }
        feature::apply_bigram(
            prev_label,
            Some(edge.label),
            provider,
            bigram_weight_indices,
            |widx| {
                gradients[usize::from_u32(widx)] -= 1.0;
            },
        );
        pos = edge.target();
        prev_label = Some(edge.label);
    }
    feature::apply_bigram(prev_label, None, provider, bigram_weight_indices, |widx| {
        gradients[usize::from_u32(widx)] -= 1.0;
    });
}
