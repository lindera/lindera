//! Module for L-BFGS optimizer.

use core::num::NonZeroU32;

use alloc::vec::Vec;

use argmin::{
    core::{CostFunction, Executor, Gradient, observers::ObserverMode},
    solver::{
        linesearch::{BacktrackingLineSearch, MoreThuenteLineSearch, condition::ArmijoCondition},
        quasinewton::LBFGS,
    },
};
use argmin_observer_slog::SlogLogger;
use hashbrown::HashMap;

use crate::feature::FeatureProvider;
use crate::lattice::Lattice;
use crate::trainer::{LatticesLoss, Regularization};

impl CostFunction for LatticesLoss<'_> {
    type Param = Vec<f64>;
    type Output = f64;

    fn cost(&self, param: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
        Ok(LatticesLoss::cost(self, param))
    }
}

impl Gradient for LatticesLoss<'_> {
    type Param = Vec<f64>;
    type Gradient = Vec<f64>;

    fn gradient(&self, param: &Self::Param) -> Result<Self::Gradient, argmin::core::Error> {
        Ok(self.gradient_partial(param, 0..self.lattices.len()))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn optimize(
    lattices: &[Lattice],
    provider: &FeatureProvider,
    unigram_weight_indices: &[Option<NonZeroU32>],
    bigram_weight_indices: &[HashMap<u32, u32>],
    weights_init: &[f64],
    regularization: Regularization,
    lambda: f64,
    max_iter: u64,
    n_threads: usize,
) -> Vec<f64> {
    let weights_init = weights_init.to_vec();
    let l2_lambda = match regularization {
        Regularization::L2 => Some(lambda),
        Regularization::ElasticNet { l1_ratio } => Some(lambda * (1.0 - l1_ratio)),
        Regularization::L1 => None,
    };
    let loss_function = LatticesLoss::new(
        lattices,
        provider,
        unigram_weight_indices,
        bigram_weight_indices,
        n_threads,
        l2_lambda,
    );
    match regularization {
        Regularization::L1 => {
            let linesearch = BacktrackingLineSearch::new(ArmijoCondition::new(1e-4).unwrap())
                .rho(0.5)
                .unwrap();
            let solver = LBFGS::new(linesearch, 7)
                .with_l1_regularization(lambda)
                .unwrap();
            let res = Executor::new(loss_function, solver)
                .configure(|state| state.param(weights_init).max_iters(max_iter))
                .add_observer(SlogLogger::term(), ObserverMode::Always)
                .run()
                .unwrap();
            res.state.param.unwrap()
        }
        Regularization::L2 => {
            let linesearch = MoreThuenteLineSearch::new().with_c(1e-4, 0.9).unwrap();
            let solver = LBFGS::new(linesearch, 7);
            let res = Executor::new(loss_function, solver)
                .configure(|state| state.param(weights_init).max_iters(max_iter))
                .add_observer(SlogLogger::term(), ObserverMode::Always)
                .run()
                .unwrap();
            res.state.param.unwrap()
        }
        Regularization::ElasticNet { l1_ratio } => {
            // OWL-QN (L1) with L2 gradient penalty via LatticesLoss
            let l1_lambda = lambda * l1_ratio;
            let linesearch = BacktrackingLineSearch::new(ArmijoCondition::new(1e-4).unwrap())
                .rho(0.5)
                .unwrap();
            let solver = LBFGS::new(linesearch, 7)
                .with_l1_regularization(l1_lambda)
                .unwrap();
            let res = Executor::new(loss_function, solver)
                .configure(|state| state.param(weights_init).max_iters(max_iter))
                .add_observer(SlogLogger::term(), ObserverMode::Always)
                .run()
                .unwrap();
            res.state.param.unwrap()
        }
    }
}
