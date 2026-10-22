//! Module for L-BFGS optimizer.

use core::num::NonZeroU32;

use alloc::vec::Vec;

use argmin::{
    core::{CostFunction, Executor, Gradient, IterState, State, observers::ObserverMode},
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
        Ok(self.gradient_partial(param))
    }
}

/// Takes the optimized parameter vector out of the final state of an L-BFGS
/// run.
///
/// A run that ends normally leaves its last parameter vector in the state, and
/// that vector is returned. When the line search fails, argmin's L-BFGS has
/// already taken the vector out of the state and stops with a `SolverExit`
/// reason; the best vector of the run is returned instead, which is the one
/// with the lowest cost and not necessarily the last point.
///
/// # Arguments
///
/// * `state` - The final state of the run.
///
/// # Returns
///
/// The parameter vector.
///
/// # Panics
///
/// Panics when the state holds neither vector, which happens only when the
/// initial cost is NaN.
fn final_param(state: &mut IterState<Vec<f64>, Vec<f64>, (), (), (), f64>) -> Vec<f64> {
    state
        .take_param()
        .or_else(|| state.take_best_param())
        .expect("L-BFGS kept no parameter vector, so the initial cost was NaN")
}

/// Minimizes the loss of the lattices with L-BFGS, or with OWL-QN when the
/// regularization has an L1 part.
///
/// # Arguments
///
/// * `lattices` - The training lattices.
/// * `provider` - The feature sets of the labels.
/// * `unigram_weight_indices` - The weight index of each unigram feature.
/// * `bigram_weight_indices` - The weight index of each bigram feature pair.
/// * `weights_init` - The initial weights.
/// * `regularization` - The kind of regularization.
/// * `lambda` - The regularization coefficient.
/// * `max_iter` - The maximum number of iterations.
/// * `n_threads` - The number of threads that evaluate the loss.
///
/// # Returns
///
/// The optimized weights, and the number of iterations that ran: `max_iter`
/// when the cap stopped the optimizer, fewer when it stopped earlier, for
/// example because it converged. argmin counts every iteration that ran,
/// including the one in which the solver stopped. When the line search of an
/// iteration fails, that iteration is counted, and the weights are the best
/// ones of the run (see [`final_param`]).
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
) -> (Vec<f64>, u64) {
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
            let mut res = Executor::new(loss_function, solver)
                .configure(|state| state.param(weights_init).max_iters(max_iter))
                .add_observer(SlogLogger::term(), ObserverMode::Always)
                .run()
                .unwrap();
            let iterations = res.state.get_iter();
            (final_param(&mut res.state), iterations)
        }
        Regularization::L2 => {
            let linesearch = MoreThuenteLineSearch::new().with_c(1e-4, 0.9).unwrap();
            let solver = LBFGS::new(linesearch, 7);
            let mut res = Executor::new(loss_function, solver)
                .configure(|state| state.param(weights_init).max_iters(max_iter))
                .add_observer(SlogLogger::term(), ObserverMode::Always)
                .run()
                .unwrap();
            let iterations = res.state.get_iter();
            (final_param(&mut res.state), iterations)
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
            let mut res = Executor::new(loss_function, solver)
                .configure(|state| state.param(weights_init).max_iters(max_iter))
                .add_observer(SlogLogger::term(), ObserverMode::Always)
                .run()
                .unwrap();
            let iterations = res.state.get_iter();
            (final_param(&mut res.state), iterations)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a state that holds `param` with `cost` and has recorded it as
    /// the best vector so far, as argmin's executor does after an iteration.
    fn state_with_best(
        param: Vec<f64>,
        cost: f64,
    ) -> IterState<Vec<f64>, Vec<f64>, (), (), (), f64> {
        let mut state = IterState::new().param(param).cost(cost);
        state.update();
        state
    }

    #[test]
    fn final_param_returns_the_last_vector_of_a_run_that_ended_normally() {
        let mut state = state_with_best(vec![1.0, 2.0], 3.0)
            .param(vec![4.0, 5.0])
            .cost(6.0);
        state.update();

        // The last vector, not the best one: a run that does not fail keeps
        // returning what it returned before the fallback existed.
        assert_eq!(final_param(&mut state), vec![4.0, 5.0]);
    }

    #[test]
    fn final_param_falls_back_to_the_best_vector_when_the_line_search_failed() {
        let mut state = state_with_best(vec![1.0, 2.0], 3.0)
            .param(vec![4.0, 5.0])
            .cost(2.0);
        state.update();
        let mut state = state.param(vec![7.0, 8.0]).cost(9.0);
        state.update();
        // argmin's L-BFGS takes the vector out of the state before the line
        // search, and does not put it back when the line search fails.
        let _ = state.take_param();

        assert_eq!(final_param(&mut state), vec![4.0, 5.0]);
    }
}
