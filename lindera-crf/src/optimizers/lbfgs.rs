//! Module for L-BFGS optimizer.

use core::cell::{Cell, RefCell};
use core::num::NonZeroU32;

use alloc::vec::Vec;

use argmin::{
    core::{
        CostFunction, Executor, Gradient, IterState, KV, Problem, Solver, State, TerminationReason,
        TerminationStatus, observers::ObserverMode,
    },
    solver::{
        linesearch::{BacktrackingLineSearch, MoreThuenteLineSearch, condition::ArmijoCondition},
        quasinewton::LBFGS,
    },
};
use argmin_observer_slog::SlogLogger;
use hashbrown::HashMap;

use crate::feature::FeatureProvider;
use crate::lattice::Lattice;
use crate::trainer::{LatticesLoss, Regularization, StopReason};

/// The loss and the gradient of one evaluation, with the parameter vector
/// they were computed at.
struct Evaluation {
    param: Vec<f64>,
    cost: f64,
    gradient: Vec<f64>,
}

/// Returns whether two parameter vectors are equal bit for bit.
///
/// `==` on `f64` treats `-0.0` and `+0.0` as equal and NaN as unequal to
/// itself, while the evaluation depends on the exact bits.
///
/// # Arguments
///
/// * `a` - A parameter vector.
/// * `b` - Another parameter vector.
///
/// # Returns
///
/// `true` when both have the same length and the same bits in every
/// component.
fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// The training objective as argmin sees it.
///
/// Each evaluation computes the loss and the gradient in one pass over the
/// lattices ([`LatticesLoss::evaluate`]), and the result for the last
/// parameter vector is kept. argmin's L-BFGS asks for the cost and the
/// gradient in separate calls, and asks for the gradient at the parameter
/// vector of the preceding cost call: in `init`, at the point the
/// backtracking line search of OWL-QN accepted (projected with the same
/// expression as in the line search), and at every trial point of the
/// More-Thuente line search, the last of which is the accepted point. The
/// gradient call is therefore answered from the kept result, and each point
/// costs one forward-backward pass instead of two or three.
pub(crate) struct CachedLoss<'a> {
    /// The objective.
    loss: LatticesLoss<'a>,
    /// The last evaluation.
    last: RefCell<Option<Evaluation>>,
    /// Counts the evaluations computed, outside the problem so that it can
    /// be read after argmin has dropped the problem.
    evaluations: &'a Cell<u64>,
}

impl<'a> CachedLoss<'a> {
    /// Creates the objective.
    ///
    /// # Arguments
    ///
    /// * `loss` - The objective to evaluate.
    /// * `evaluations` - The counter of the evaluations computed.
    ///
    /// # Returns
    ///
    /// The objective, with nothing evaluated yet.
    pub(crate) const fn new(loss: LatticesLoss<'a>, evaluations: &'a Cell<u64>) -> Self {
        Self {
            loss,
            last: RefCell::new(None),
            evaluations,
        }
    }

    /// Calls `f` with the evaluation at `param`, computing it unless it is
    /// the kept one.
    ///
    /// # Arguments
    ///
    /// * `param` - The parameter vector.
    /// * `f` - Reads what the caller needs from the evaluation.
    ///
    /// # Returns
    ///
    /// What `f` returns.
    fn with_evaluation<T>(&self, param: &[f64], f: impl FnOnce(&Evaluation) -> T) -> T {
        let mut last = self.last.borrow_mut();
        let evaluation = match last.take() {
            Some(kept) if same_bits(&kept.param, param) => kept,
            _ => {
                let (cost, gradient) = self.loss.evaluate(param);
                self.evaluations.set(self.evaluations.get() + 1);
                Evaluation {
                    param: param.to_vec(),
                    cost,
                    gradient,
                }
            }
        };
        let result = f(&evaluation);
        *last = Some(evaluation);
        result
    }
}

impl CostFunction for CachedLoss<'_> {
    type Param = Vec<f64>;
    type Output = f64;

    fn cost(&self, param: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
        Ok(self.with_evaluation(param, |evaluation| evaluation.cost))
    }
}

impl Gradient for CachedLoss<'_> {
    type Param = Vec<f64>;
    type Gradient = Vec<f64>;

    fn gradient(&self, param: &Self::Param) -> Result<Self::Gradient, argmin::core::Error> {
        Ok(self.with_evaluation(param, |evaluation| evaluation.gradient.clone()))
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

/// Returns the L2 strength the objective adds itself: all of it for L2, the
/// L2 part for Elastic Net, none for L1, whose penalty OWL-QN applies.
///
/// # Arguments
///
/// * `regularization` - The kind of regularization.
/// * `lambda` - The regularization coefficient.
///
/// # Returns
///
/// The L2 strength, if any.
pub(crate) fn l2_lambda(regularization: Regularization, lambda: f64) -> Option<f64> {
    match regularization {
        Regularization::L2 => Some(lambda),
        Regularization::ElasticNet { l1_ratio } => Some(lambda * (1.0 - l1_ratio)),
        Regularization::L1 => None,
    }
}

/// MeCab's convergence test: the relative change of the cost stays below a
/// tolerance for [`Self::STREAK`] iterations in a row.
struct RelativeChange {
    /// The tolerance; `0.0` never passes.
    eta: f64,
    /// The number of iterations in a row that passed so far.
    passed: u32,
}

impl RelativeChange {
    /// The number of iterations in a row that must pass; MeCab's learner
    /// uses 3 too.
    const STREAK: u32 = 3;

    /// Creates the test.
    ///
    /// # Arguments
    ///
    /// * `eta` - The tolerance.
    ///
    /// # Returns
    ///
    /// The test, with no iteration passed.
    const fn new(eta: f64) -> Self {
        Self { eta, passed: 0 }
    }

    /// Records the cost of one more iteration.
    ///
    /// The iteration passes when `|prev - cost| / |prev| < eta`. An infinite
    /// or zero previous cost, or a NaN cost, gives an infinite or NaN ratio,
    /// which fails the comparison; so does the first call, whose previous
    /// cost argmin keeps at infinity. A failing iteration restarts the count.
    ///
    /// # Arguments
    ///
    /// * `prev` - The cost of the previous iteration.
    /// * `cost` - The cost of this iteration.
    ///
    /// # Returns
    ///
    /// `true` once [`Self::STREAK`] iterations in a row have passed.
    fn record(&mut self, prev: f64, cost: f64) -> bool {
        let passes = (prev - cost).abs() / prev.abs() < self.eta;
        self.passed = if passes { self.passed + 1 } else { 0 };
        self.passed >= Self::STREAK
    }
}

/// How the wrapped solver's run ended, as far as [`ConvergenceTest`] saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Convergence {
    /// The relative change passed, or argmin's L-BFGS found a vanishing
    /// gradient.
    Converged,
    /// argmin's L-BFGS stopped because an iteration left the cost unchanged.
    NoProgress,
}

/// The gradient norm under which argmin's L-BFGS stops, its default
/// `tol_grad`.
fn argmin_tol_grad() -> f64 {
    f64::EPSILON.sqrt()
}

/// argmin's L-BFGS with the convergence test of `lindera train` added.
///
/// argmin's L-BFGS stops when the norm of the gradient falls below `√ε` or
/// when the cost changes by less than `ε` in absolute terms. For a training
/// loss in the hundreds or thousands the first practically never holds, and
/// the second only when an iteration left the cost unchanged. This solver
/// keeps both, records which of them stopped the run, and adds MeCab's test
/// on the relative change ([`RelativeChange`]). It delegates everything else
/// to the wrapped solver, so the steps and the log lines are argmin's.
struct ConvergenceTest<S> {
    /// The wrapped L-BFGS solver.
    solver: S,
    /// The relative-change test.
    relative: RelativeChange,
    /// Set when one of the tests stopped the run.
    convergence: Option<Convergence>,
}

impl<S> ConvergenceTest<S> {
    /// Wraps a solver.
    ///
    /// # Arguments
    ///
    /// * `solver` - argmin's L-BFGS solver.
    /// * `eta` - The tolerance of the relative-change test.
    ///
    /// # Returns
    ///
    /// The wrapped solver.
    const fn new(solver: S, eta: f64) -> Self {
        Self {
            solver,
            relative: RelativeChange::new(eta),
            convergence: None,
        }
    }
}

/// The state of every L-BFGS run here.
type LbfgsState = IterState<Vec<f64>, Vec<f64>, (), (), (), f64>;

impl<O, S> Solver<O, LbfgsState> for ConvergenceTest<S>
where
    S: Solver<O, LbfgsState>,
{
    fn name(&self) -> &str {
        self.solver.name()
    }

    fn init(
        &mut self,
        problem: &mut Problem<O>,
        state: LbfgsState,
    ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
        self.solver.init(problem, state)
    }

    fn next_iter(
        &mut self,
        problem: &mut Problem<O>,
        state: LbfgsState,
    ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
        self.solver.next_iter(problem, state)
    }

    fn terminate(&mut self, state: &LbfgsState) -> TerminationStatus {
        let status = self.solver.terminate(state);
        if status.terminated() {
            let gradient_vanished = state
                .get_gradient()
                .is_some_and(|g| g.iter().map(|x| x * x).sum::<f64>().sqrt() < argmin_tol_grad());
            self.convergence = Some(if gradient_vanished {
                Convergence::Converged
            } else {
                Convergence::NoProgress
            });
            return status;
        }
        if self
            .relative
            .record(state.get_prev_cost(), state.get_cost())
        {
            self.convergence = Some(Convergence::Converged);
            return TerminationStatus::Terminated(TerminationReason::SolverConverged);
        }
        TerminationStatus::NotTerminated
    }
}

/// Tells why a run stopped, from its final termination status and what
/// [`ConvergenceTest`] recorded.
///
/// The run sets no target cost and no timeout, and turns argmin's Ctrl-C
/// handling off, so it cannot end with `TargetCostReached`, `Timeout` or
/// `Interrupt`; those statuses, which would also come with a `None`, are
/// reported as [`StopReason::Converged`] only because nothing else is left.
///
/// # Arguments
///
/// * `status` - The final termination status.
/// * `convergence` - What the convergence tests recorded.
///
/// # Returns
///
/// The reason.
fn stop_reason(status: &TerminationStatus, convergence: Option<Convergence>) -> StopReason {
    match status {
        TerminationStatus::Terminated(TerminationReason::MaxItersReached) => {
            StopReason::MaxIterations
        }
        TerminationStatus::Terminated(TerminationReason::SolverExit(_)) => {
            StopReason::LineSearchFailed
        }
        _ => match convergence {
            Some(Convergence::NoProgress) => StopReason::NoProgress,
            Some(Convergence::Converged) | None => StopReason::Converged,
        },
    }
}

/// The result of [`optimize`].
pub struct Optimized {
    /// The optimized weights.
    pub weights: Vec<f64>,
    /// The number of iterations that ran, as described for [`minimize`].
    pub iterations: u64,
    /// The number of evaluations of the loss and its gradient: one for every
    /// point at which argmin asked for either of them.
    pub evaluations: u64,
    /// Why the optimizer stopped.
    pub stop_reason: StopReason,
}

/// The result of [`minimize`].
pub(crate) struct Minimized {
    /// The optimized weights.
    pub weights: Vec<f64>,
    /// The number of iterations that ran, as described for [`minimize`].
    pub iterations: u64,
    /// Why the optimizer stopped.
    pub stop_reason: StopReason,
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
/// * `eta` - The tolerance of the convergence test (see
///   [`ConvergenceTest`]).
/// * `n_threads` - The number of threads that evaluate the loss.
///
/// # Returns
///
/// The optimized weights, the number of iterations that ran, the number of
/// evaluations and why the optimizer stopped.
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
    eta: f64,
    n_threads: usize,
) -> Optimized {
    let loss = LatticesLoss::new(
        lattices,
        provider,
        unigram_weight_indices,
        bigram_weight_indices,
        n_threads,
        l2_lambda(regularization, lambda),
    );
    let evaluations = Cell::new(0);
    let Minimized {
        weights,
        iterations,
        stop_reason,
    } = minimize(
        CachedLoss::new(loss, &evaluations),
        weights_init.to_vec(),
        regularization,
        lambda,
        max_iter,
        eta,
    );
    Optimized {
        weights,
        iterations,
        evaluations: evaluations.get(),
        stop_reason,
    }
}

/// Runs a wrapped L-BFGS solver on `problem` from `weights_init`.
///
/// # Arguments
///
/// * `problem` - The objective.
/// * `solver` - argmin's L-BFGS solver, with its line search.
/// * `weights_init` - The initial weights.
/// * `max_iter` - The maximum number of iterations.
/// * `eta` - The tolerance of the convergence test.
///
/// # Returns
///
/// The result, as described for [`minimize`].
fn run<P, S>(problem: P, solver: S, weights_init: Vec<f64>, max_iter: u64, eta: f64) -> Minimized
where
    S: Solver<P, LbfgsState>,
{
    let mut res = Executor::new(problem, ConvergenceTest::new(solver, eta))
        .configure(|state| state.param(weights_init).max_iters(max_iter))
        .ctrlc(false)
        .add_observer(SlogLogger::term(), ObserverMode::Always)
        .run()
        .unwrap();
    let iterations = res.state.get_iter();
    let stop_reason = stop_reason(res.state.get_termination_status(), res.solver.convergence);
    Minimized {
        weights: final_param(&mut res.state),
        iterations,
        stop_reason,
    }
}

/// Runs L-BFGS, or OWL-QN when the regularization has an L1 part, on
/// `problem` from `weights_init`.
///
/// # Arguments
///
/// * `problem` - The objective, which must already include the L2 part of
///   the regularization (see [`l2_lambda`]).
/// * `weights_init` - The initial weights.
/// * `regularization` - The kind of regularization.
/// * `lambda` - The regularization coefficient.
/// * `max_iter` - The maximum number of iterations.
/// * `eta` - The tolerance of the convergence test (see
///   [`ConvergenceTest`]).
///
/// # Returns
///
/// The optimized weights, the number of iterations that ran, and why the
/// optimizer stopped. The iterations are `max_iter` when the cap stopped the
/// optimizer, fewer when it stopped earlier, for example because it
/// converged. argmin counts every iteration that ran, including the one in
/// which the solver stopped. When the line search of an iteration fails,
/// that iteration is counted, and the weights are the best ones of the run
/// (see [`final_param`]).
pub(crate) fn minimize<P>(
    problem: P,
    weights_init: Vec<f64>,
    regularization: Regularization,
    lambda: f64,
    max_iter: u64,
    eta: f64,
) -> Minimized
where
    P: CostFunction<Param = Vec<f64>, Output = f64>
        + Gradient<Param = Vec<f64>, Gradient = Vec<f64>>,
{
    match regularization {
        Regularization::L1 => {
            let linesearch = BacktrackingLineSearch::new(ArmijoCondition::new(1e-4).unwrap())
                .rho(0.5)
                .unwrap();
            let solver = LBFGS::new(linesearch, 7)
                .with_l1_regularization(lambda)
                .unwrap();
            run(problem, solver, weights_init, max_iter, eta)
        }
        Regularization::L2 => {
            let linesearch = MoreThuenteLineSearch::new().with_c(1e-4, 0.9).unwrap();
            let solver = LBFGS::new(linesearch, 7);
            run(problem, solver, weights_init, max_iter, eta)
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
            run(problem, solver, weights_init, max_iter, eta)
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

    #[test]
    fn relative_change_passes_after_three_iterations_in_a_row() {
        let mut test = RelativeChange::new(1e-3);
        // The first check: argmin keeps the previous cost at infinity.
        assert!(!test.record(f64::INFINITY, 100.0));
        assert!(!test.record(100.0, 99.95));
        assert!(!test.record(99.95, 99.9));
        // A large change restarts the count...
        assert!(!test.record(99.9, 90.0));
        assert!(!test.record(90.0, 89.99));
        assert!(!test.record(89.99, 89.98));
        // ...and the third small change in a row passes.
        assert!(test.record(89.98, 89.97));
    }

    #[test]
    fn relative_change_needs_a_finite_nonzero_previous_cost() {
        for prev in [f64::INFINITY, f64::NAN, 0.0] {
            let mut test = RelativeChange::new(1.0);
            for _ in 0..5 {
                assert!(!test.record(prev, 0.0), "previous cost {prev}");
            }
        }
        // A NaN ratio restarts the count.
        let mut test = RelativeChange::new(1.0);
        assert!(!test.record(1.0, 1.0));
        assert!(!test.record(1.0, 1.0));
        assert!(!test.record(1.0, f64::NAN));
        assert!(!test.record(1.0, 1.0));
        assert!(!test.record(1.0, 1.0));
        assert!(test.record(1.0, 1.0));
    }

    #[test]
    fn relative_change_with_eta_zero_never_passes() {
        let mut test = RelativeChange::new(0.0);
        for _ in 0..10 {
            assert!(!test.record(1.0, 1.0));
        }
    }

    /// A solver whose own test stops the run at once, standing in for
    /// argmin's L-BFGS when one of its tests holds.
    struct StopsAtOnce;

    impl<O> Solver<O, LbfgsState> for StopsAtOnce {
        fn name(&self) -> &str {
            "stops at once"
        }

        fn next_iter(
            &mut self,
            _problem: &mut Problem<O>,
            state: LbfgsState,
        ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
            Ok((state, None))
        }

        fn terminate(&mut self, _state: &LbfgsState) -> TerminationStatus {
            TerminationStatus::Terminated(TerminationReason::SolverConverged)
        }
    }

    #[test]
    fn a_stop_by_argmin_is_told_apart_by_the_gradient() {
        let classify = |gradient: Vec<f64>| {
            let mut test = ConvergenceTest::new(StopsAtOnce, 0.0);
            let state = LbfgsState::new().param(vec![0.0, 0.0]).gradient(gradient);
            let status = Solver::<(), LbfgsState>::terminate(&mut test, &state);
            assert!(status.terminated());
            test.convergence
        };
        // A gradient norm below argmin's tolerance: converged.
        assert_eq!(classify(vec![1e-9, 0.0]), Some(Convergence::Converged));
        // Otherwise argmin stopped because the cost did not change.
        assert_eq!(classify(vec![1.0, 0.0]), Some(Convergence::NoProgress));
    }

    #[test]
    fn stop_reason_follows_the_termination_status() {
        let terminated = TerminationStatus::Terminated;
        assert_eq!(
            stop_reason(&terminated(TerminationReason::MaxItersReached), None),
            StopReason::MaxIterations
        );
        assert_eq!(
            stop_reason(
                &terminated(TerminationReason::SolverExit(
                    "Line search terminated with: ...".into()
                )),
                None
            ),
            StopReason::LineSearchFailed
        );
        assert_eq!(
            stop_reason(
                &terminated(TerminationReason::SolverConverged),
                Some(Convergence::Converged)
            ),
            StopReason::Converged
        );
        assert_eq!(
            stop_reason(
                &terminated(TerminationReason::SolverConverged),
                Some(Convergence::NoProgress)
            ),
            StopReason::NoProgress
        );
    }
}
