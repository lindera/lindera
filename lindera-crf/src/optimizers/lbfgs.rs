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
        linesearch::{
            BacktrackingLineSearch, LineSearch, MoreThuenteLineSearch, condition::ArmijoCondition,
        },
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
    /// The relative change passed, or argmin's L-BFGS stopped near the
    /// optimum ([`ConvergenceTest::near_optimum`]), whether by its own tests
    /// or because a line search failed.
    Converged,
    /// argmin's L-BFGS stopped far from the optimum because an iteration
    /// left the cost unchanged.
    NoProgress,
}

/// The gradient norm under which argmin's L-BFGS stops, its default
/// `tol_grad`. A gradient under it is near the optimum whatever the initial
/// gradient ([`ConvergenceTest::near_optimum`]).
fn argmin_tol_grad() -> f64 {
    f64::EPSILON.sqrt()
}

/// The ratio of a gradient norm to the one at the initial weights under
/// which the weights are near the optimum ([`ConvergenceTest::near_optimum`]):
/// `1e-5`, the default `epsilon` of libLBFGS's gradient test
/// (`lib/lbfgs.c` at `5ad02fbe`, lines 114 and 505–508).
///
/// The loss is a sum over the lattices, so its gradient grows with the
/// corpus, while the ratio does not. The runs of #1300 that stopped near the
/// optimum did so at 2.2e-8 of the initial gradient and below, and the stops
/// far from it known, of the wide fixture before #1294 and of the quartic in
/// the tests, at 1.7e-4 and above.
const NEAR_OPTIMUM_GRADIENT_RATIO: f64 = 1e-5;

/// Returns the Euclidean norm of a vector.
///
/// # Arguments
///
/// * `v` - The vector.
///
/// # Returns
///
/// The norm.
fn norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// argmin's L-BFGS with the convergence test of `lindera train` added.
///
/// argmin's L-BFGS stops when the norm of the gradient falls below `√ε` or
/// when the cost changes by less than `ε` in absolute terms, and when a line
/// search fails. Near the optimum the loss changes by less than its
/// rounding, and which of these comes first is decided by the last bits of
/// the libm results, which differ between platforms (#1289, #1300). This
/// solver therefore tells a stop by where it leaves the weights, not by
/// which test held: near the optimum ([`Self::near_optimum`]) the run
/// converged, and only a stop far from it is reported as one that made no
/// progress or whose line search failed. It adds MeCab's test on the
/// relative change ([`RelativeChange`]), and delegates everything else to
/// the wrapped solver, so the steps and the log lines are argmin's.
struct ConvergenceTest<S> {
    /// The wrapped L-BFGS solver.
    solver: S,
    /// The relative-change test.
    relative: RelativeChange,
    /// The norm of the gradient that argmin's `init` leaves in the state: at
    /// the initial weights, without the L1 term for OWL-QN. It sets the
    /// scale of the problem for [`Self::near_optimum`]; 0 until `init`.
    initial_gradient_norm: f64,
    /// The lowest cost of the run so far.
    best_cost: f64,
    /// The norm of the gradient at the weights of [`Self::best_cost`], which
    /// a run whose line search failed returns (see [`final_param`]); NaN
    /// until [`Solver::terminate`] has seen them.
    best_gradient_norm: f64,
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
            initial_gradient_norm: 0.0,
            best_cost: f64::INFINITY,
            best_gradient_norm: f64::NAN,
            convergence: None,
        }
    }

    /// Returns whether weights with a gradient of the given norm are near
    /// the optimum: the norm is under argmin's `tol_grad`, or under
    /// [`NEAR_OPTIMUM_GRADIENT_RATIO`] times the norm at the initial
    /// weights. A NaN norm is not.
    ///
    /// # Arguments
    ///
    /// * `gradient_norm` - The norm of the gradient (the pseudo-gradient for
    ///   OWL-QN) at the weights.
    ///
    /// # Returns
    ///
    /// `true` when the weights are near the optimum.
    fn near_optimum(&self, gradient_norm: f64) -> bool {
        gradient_norm < argmin_tol_grad()
            || gradient_norm < NEAR_OPTIMUM_GRADIENT_RATIO * self.initial_gradient_norm
    }
}

/// Returns the norm of the gradient in a state, or NaN when the state holds
/// none.
///
/// # Arguments
///
/// * `state` - The state.
///
/// # Returns
///
/// The norm.
fn gradient_norm(state: &LbfgsState) -> f64 {
    state
        .get_gradient()
        .map_or(f64::NAN, |gradient| norm(gradient))
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

    /// Initializes the wrapped solver, and records the norm of the gradient
    /// it leaves in the state.
    fn init(
        &mut self,
        problem: &mut Problem<O>,
        state: LbfgsState,
    ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
        let (state, kv) = self.solver.init(problem, state)?;
        self.initial_gradient_norm = gradient_norm(&state);
        Ok((state, kv))
    }

    /// Runs an iteration of the wrapped solver. When its line search fails,
    /// argmin's L-BFGS ends the run here, and the executor does not call
    /// [`Solver::terminate`] again, so the run is classified here, by the
    /// gradient at the best weights, which it returns.
    fn next_iter(
        &mut self,
        problem: &mut Problem<O>,
        state: LbfgsState,
    ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
        let (state, kv) = self.solver.next_iter(problem, state)?;
        if matches!(
            state.get_termination_status(),
            TerminationStatus::Terminated(TerminationReason::SolverExit(_))
        ) && self.near_optimum(self.best_gradient_norm)
        {
            self.convergence = Some(Convergence::Converged);
        }
        Ok((state, kv))
    }

    /// Records the gradient at the best weights so far, and classifies a
    /// stop by argmin's own tests by the gradient in the state.
    ///
    /// The executor calls this after every update of the state, which makes
    /// the current weights the best ones only on a strictly lower cost; a
    /// lower best cost therefore means that the gradient in the state is the
    /// one at the best weights.
    fn terminate(&mut self, state: &LbfgsState) -> TerminationStatus {
        let current = gradient_norm(state);
        if state.get_best_cost() < self.best_cost {
            self.best_cost = state.get_best_cost();
            self.best_gradient_norm = current;
        }
        let status = self.solver.terminate(state);
        if status.terminated() {
            self.convergence = Some(if self.near_optimum(current) {
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

/// A line search whose first search starts at a step of `1 / |d|`, `d` being
/// its search direction, and every later search at a step of 1.
///
/// argmin's L-BFGS has no curvature pair in its first iteration and searches
/// along the negative gradient (the negative pseudo-gradient, projected, for
/// OWL-QN), and both of its line searches start at a step of 1. The first
/// trial point therefore lies as far from the initial weights as the gradient
/// is long: 1.1e5 on the
/// corpus of #1260, where the first search went so far that the loss was
/// rounding error and OWL-QN stopped after 3 iterations (#1290). As in
/// MeCab's learner (`mecab/src/lbfgs.cpp` at `61b90ba6`, lines 463 and
/// 531–534) and libLBFGS (`lib/lbfgs.c` at `5ad02fbe`, lines 436–460 and
/// 618), the first search starts at `1 / |d|`, so that its first trial point
/// lies at unit distance. For L-BFGS `|d|` is the length of the gradient, as
/// in both; for OWL-QN it is that of the projected pseudo-gradient, as in
/// libLBFGS, where MeCab takes the length of the gradient without the L1
/// term.
///
/// argmin's L-BFGS calls [`LineSearch::search_direction`] on its own line
/// search and then runs a clone of it, so the step is recorded with the
/// direction and handed to the inner line search in [`Solver::init`], on the
/// clone. The More-Thuente line search clamps its steps to `√ε` and more, so
/// for a direction longer than about 6.7e7 the first step is `√ε`.
#[derive(Clone)]
struct UnitFirstStep<L> {
    /// The wrapped line search.
    inner: L,
    /// The step the coming search starts at.
    step: f64,
    /// Set once the first search direction has been given.
    started: bool,
}

impl<L> UnitFirstStep<L> {
    /// Wraps a line search.
    ///
    /// # Arguments
    ///
    /// * `inner` - The line search to wrap.
    ///
    /// # Returns
    ///
    /// The wrapped line search, before its first search.
    const fn new(inner: L) -> Self {
        Self {
            inner,
            step: 1.0,
            started: false,
        }
    }
}

impl<L> LineSearch<Vec<f64>, f64> for UnitFirstStep<L>
where
    L: LineSearch<Vec<f64>, f64>,
{
    /// Sets the search direction, and the step the search starts at:
    /// `1 / |direction|` for the first search, 1 for every later one. A
    /// direction whose length is 0 or not finite gets 1 too.
    fn search_direction(&mut self, direction: Vec<f64>) {
        self.step = 1.0;
        if !self.started {
            self.started = true;
            let length = norm(&direction);
            if length.is_finite() && length > 0.0 {
                self.step = 1.0 / length;
            }
        }
        self.inner.search_direction(direction);
    }

    /// Sets the step the coming search starts at, which the inner line search
    /// checks.
    fn initial_step_length(&mut self, step_length: f64) -> Result<(), argmin::core::Error> {
        self.inner.initial_step_length(step_length)?;
        self.step = step_length;
        Ok(())
    }
}

impl<O, L> Solver<O, LbfgsState> for UnitFirstStep<L>
where
    L: LineSearch<Vec<f64>, f64> + Solver<O, LbfgsState>,
{
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn init(
        &mut self,
        problem: &mut Problem<O>,
        state: LbfgsState,
    ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
        self.inner.initial_step_length(self.step)?;
        self.inner.init(problem, state)
    }

    fn next_iter(
        &mut self,
        problem: &mut Problem<O>,
        state: LbfgsState,
    ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
        self.inner.next_iter(problem, state)
    }

    fn terminate(&mut self, state: &LbfgsState) -> TerminationStatus {
        self.inner.terminate(state)
    }
}

/// Tells why a run stopped, from its final termination status and what
/// [`ConvergenceTest`] recorded.
///
/// A failed line search (`SolverExit`) is [`StopReason::Converged`] when
/// [`ConvergenceTest`] found the best weights near the optimum, and
/// [`StopReason::LineSearchFailed`] otherwise.
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
        TerminationStatus::Terminated(TerminationReason::SolverExit(_)) => match convergence {
            Some(Convergence::Converged) => StopReason::Converged,
            _ => StopReason::LineSearchFailed,
        },
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
    let l1_lambda = match regularization {
        Regularization::L1 => lambda,
        Regularization::L2 => 0.0,
        Regularization::ElasticNet { l1_ratio } => lambda * l1_ratio,
    };
    // OWL-QN requires a positive L1 coefficient. With no L1 penalty, use
    // ordinary L-BFGS; the objective already includes any L2 penalty. Either
    // way the first line search starts at a unit step (see UnitFirstStep).
    if l1_lambda > 0.0 {
        let linesearch = BacktrackingLineSearch::new(ArmijoCondition::new(1e-4).unwrap())
            .rho(0.5)
            .unwrap();
        let solver = LBFGS::new(UnitFirstStep::new(linesearch), 7)
            .with_l1_regularization(l1_lambda)
            .unwrap();
        run(problem, solver, weights_init, max_iter, eta)
    } else {
        let linesearch = MoreThuenteLineSearch::new().with_c(1e-4, 0.9).unwrap();
        let solver = LBFGS::new(UnitFirstStep::new(linesearch), 7);
        run(problem, solver, weights_init, max_iter, eta)
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
        // Initializes the test with `initial` as the gradient at the initial
        // weights, then stops with `gradient`.
        let classify = |initial: Vec<f64>, gradient: Vec<f64>| {
            let mut test = ConvergenceTest::new(StopsAtOnce, 0.0);
            let state = LbfgsState::new().param(vec![0.0, 0.0]).gradient(initial);
            let (state, _) =
                Solver::<(), LbfgsState>::init(&mut test, &mut Problem::new(()), state).unwrap();
            let state = state.gradient(gradient);
            let status = Solver::<(), LbfgsState>::terminate(&mut test, &state);
            assert!(status.terminated());
            test.convergence
        };
        // Under 1e-5 times the initial gradient, though far over argmin's
        // tolerance: near the optimum, converged.
        assert_eq!(
            classify(vec![3e3, 4e3], vec![0.03, 0.0]),
            Some(Convergence::Converged)
        );
        // Under argmin's tolerance: converged, however small the initial
        // gradient.
        assert_eq!(
            classify(vec![1e-6, 0.0], vec![1e-9, 0.0]),
            Some(Convergence::Converged)
        );
        // Otherwise argmin stopped far from the optimum because the cost did
        // not change.
        assert_eq!(
            classify(vec![3e3, 4e3], vec![0.06, 0.0]),
            Some(Convergence::NoProgress)
        );
    }

    /// A solver that moves, iteration by iteration, to points of the given
    /// costs and gradients, and then fails as argmin's L-BFGS does when its
    /// line search fails: it takes the weights and the gradient out of the
    /// state and ends the run with `SolverExit`.
    struct StepsThenFails {
        steps: Vec<(f64, Vec<f64>)>,
    }

    impl<O> Solver<O, LbfgsState> for StepsThenFails {
        fn name(&self) -> &str {
            "steps then fails"
        }

        fn next_iter(
            &mut self,
            _problem: &mut Problem<O>,
            mut state: LbfgsState,
        ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
            if self.steps.is_empty() {
                let _ = state.take_param();
                let _ = state.take_gradient();
                let reason =
                    TerminationReason::SolverExit("Line search terminated with: ...".into());
                return Ok((state.terminate_with(reason), None));
            }
            let (cost, gradient) = self.steps.remove(0);
            Ok((state.cost(cost).gradient(gradient), None))
        }
    }

    #[test]
    fn a_failed_line_search_is_told_apart_by_the_gradient_at_the_best_weights() {
        // From a cost of 10 and an initial gradient 5e3 long, the run moves
        // to the best weights, at a cost of 1, then to a point of cost 2,
        // whose line search fails. The run returns the best weights, so their
        // gradient decides.
        let run = |best: Vec<f64>, last: Vec<f64>| {
            let solver = StepsThenFails {
                steps: vec![(1.0, best), (2.0, last)],
            };
            let res = Executor::new((), ConvergenceTest::new(solver, 0.0))
                .configure(|state| {
                    state
                        .param(vec![0.0, 0.0])
                        .gradient(vec![3e3, 4e3])
                        .cost(10.0)
                })
                .ctrlc(false)
                .run()
                .unwrap();
            stop_reason(res.state.get_termination_status(), res.solver.convergence)
        };
        assert_eq!(run(vec![0.03, 0.0], vec![1.0, 0.0]), StopReason::Converged);
        assert_eq!(
            run(vec![1.0, 0.0], vec![0.03, 0.0]),
            StopReason::LineSearchFailed
        );
    }

    /// A line search that records the step each search starts at, in a list
    /// that its clones share, standing in for argmin's line searches.
    #[derive(Clone, Default)]
    struct RecordsSteps {
        steps: alloc::rc::Rc<RefCell<Vec<f64>>>,
    }

    impl LineSearch<Vec<f64>, f64> for RecordsSteps {
        fn search_direction(&mut self, _direction: Vec<f64>) {}

        fn initial_step_length(&mut self, step_length: f64) -> Result<(), argmin::core::Error> {
            self.steps.borrow_mut().push(step_length);
            Ok(())
        }
    }

    impl Solver<(), LbfgsState> for RecordsSteps {
        fn name(&self) -> &str {
            "records steps"
        }

        fn next_iter(
            &mut self,
            _problem: &mut Problem<()>,
            state: LbfgsState,
        ) -> Result<(LbfgsState, Option<KV>), argmin::core::Error> {
            Ok((state, None))
        }
    }

    /// Runs one search along `direction` as argmin's L-BFGS does: sets the
    /// direction on its own line search, then initializes a clone of it.
    fn search(line_search: &mut UnitFirstStep<RecordsSteps>, direction: Vec<f64>) {
        line_search.search_direction(direction);
        let mut clone = line_search.clone();
        Solver::<(), LbfgsState>::init(&mut clone, &mut Problem::new(()), LbfgsState::new())
            .unwrap();
    }

    #[test]
    fn the_first_search_starts_at_one_over_the_length_of_the_direction() {
        let inner = RecordsSteps::default();
        let steps = alloc::rc::Rc::clone(&inner.steps);
        let mut line_search = UnitFirstStep::new(inner);

        search(&mut line_search, vec![3.0, 4.0]);
        search(&mut line_search, vec![30.0, 40.0]);
        search(&mut line_search, vec![0.3, 0.4]);

        // 1 / |[3, 4]| for the first search, 1 for the later ones.
        assert_eq!(*steps.borrow(), vec![0.2, 1.0, 1.0]);
    }

    #[test]
    fn a_first_direction_of_no_finite_length_starts_at_one() {
        // The length of [1e200, 1e200] overflows.
        let wrong: Vec<(Vec<f64>, Vec<f64>)> = [
            vec![0.0, 0.0],
            vec![f64::NAN, 1.0],
            vec![f64::INFINITY, 1.0],
            vec![1e200, 1e200],
        ]
        .into_iter()
        .filter_map(|direction| {
            let inner = RecordsSteps::default();
            let steps = alloc::rc::Rc::clone(&inner.steps);
            let mut line_search = UnitFirstStep::new(inner);
            search(&mut line_search, direction.clone());
            let steps = steps.borrow().clone();
            (steps != [1.0]).then_some((direction, steps))
        })
        .collect();

        assert!(wrong.is_empty(), "(direction, steps): {wrong:?}");
    }

    /// `scale / 2 * |w - center|²`, which records every point at which its
    /// cost or gradient is asked for.
    struct RecordedQuadratic<'a> {
        scale: f64,
        center: Vec<f64>,
        points: &'a RefCell<Vec<Vec<f64>>>,
    }

    impl CostFunction for RecordedQuadratic<'_> {
        type Param = Vec<f64>;
        type Output = f64;

        fn cost(&self, param: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
            self.points.borrow_mut().push(param.clone());
            let squared: f64 = param
                .iter()
                .zip(&self.center)
                .map(|(w, c)| (w - c) * (w - c))
                .sum();
            Ok(self.scale / 2.0 * squared)
        }
    }

    impl Gradient for RecordedQuadratic<'_> {
        type Param = Vec<f64>;
        type Gradient = Vec<f64>;

        fn gradient(&self, param: &Self::Param) -> Result<Self::Gradient, argmin::core::Error> {
            self.points.borrow_mut().push(param.clone());
            Ok(param
                .iter()
                .zip(&self.center)
                .map(|(w, c)| self.scale * (w - c))
                .collect())
        }
    }

    /// Returns how far from the origin the first point other than the origin
    /// lies at which `minimize` evaluates a quadratic whose gradient at the
    /// origin is 5e4 long, starting from the origin.
    fn first_trial_distance(regularization: Regularization, lambda: f64) -> f64 {
        let points = RefCell::new(Vec::new());
        let problem = RecordedQuadratic {
            scale: 1e4,
            center: vec![3.0, 4.0],
            points: &points,
        };
        minimize(problem, vec![0.0, 0.0], regularization, lambda, 1, 0.0);
        let points = points.into_inner();
        let trial = points
            .iter()
            .find(|point| point.iter().any(|x| *x != 0.0))
            .expect("minimize evaluated only the origin");
        trial.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    #[test]
    fn the_first_trial_point_of_l_bfgs_lies_at_unit_distance() {
        // More-Thuente starting at a step of 1 would first try the point 5e4
        // away.
        let distance = first_trial_distance(Regularization::L2, 0.0);
        assert!((distance - 1.0).abs() < 1e-12, "distance {distance}");
    }

    #[test]
    fn the_first_trial_point_of_owl_qn_lies_at_unit_distance() {
        // The pseudo-gradient at the origin is the gradient moved by lambda
        // toward 0; backtracking from a step of 1 would first try it whole.
        let distance = first_trial_distance(Regularization::L1, 1.0);
        assert!((distance - 1.0).abs() < 1e-12, "distance {distance}");
    }

    /// `w⁴ / 4 − w²` of a single weight, concave on (−√(2/3), √(2/3)).
    ///
    /// It is computed with `+`, `−`, `×` and `÷` only, whose results IEEE 754
    /// fixes, unlike those of libm's `exp` and `log1p`, so a run on it is the
    /// same on every platform (#1289).
    struct Quartic;

    impl CostFunction for Quartic {
        type Param = Vec<f64>;
        type Output = f64;

        fn cost(&self, param: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
            let w = param[0];
            Ok(w * w * w * w / 4.0 - w * w)
        }
    }

    impl Gradient for Quartic {
        type Param = Vec<f64>;
        type Gradient = Vec<f64>;

        fn gradient(&self, param: &Self::Param) -> Result<Self::Gradient, argmin::core::Error> {
            let w = param[0];
            Ok(vec![w * w * w - 2.0 * w])
        }
    }

    // An iteration that cannot move the weights leaves the cost unchanged and
    // argmin's L-BFGS stops; the reason says so instead of claiming
    // convergence (#1260). From 0.1, OWL-QN's first step goes a unit distance,
    // to 1.1. The quartic is concave in between, so the gradient falls, from
    // −0.199 to −0.869, and the curvature pair has `s·y < 0`. The inverse
    // Hessian that L-BFGS builds from it, `s / y` in one dimension, is
    // negative, so the next direction goes against the pseudo-gradient, and
    // OWL-QN's sign projection zeroes it. The second iteration leaves the
    // weight where it is, with a pseudo-gradient of −0.859, far from argmin's
    // tolerance. The L1 run of the wide fixture stopped the same way on Linux
    // before #1294, with a curvature pair of rounding error that came out
    // otherwise on macOS, so a test on that run depended on the platform
    // (#1289).
    #[test]
    fn an_unchanged_objective_is_reported_as_no_progress() {
        let result = minimize(Quartic, vec![0.1], Regularization::L1, 0.01, 100, 0.0);
        assert_eq!(result.stop_reason, StopReason::NoProgress);
        assert_eq!(result.iterations, 2);
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
        // A line search that failed near the optimum.
        assert_eq!(
            stop_reason(
                &terminated(TerminationReason::SolverExit(
                    "Line search terminated with: ...".into()
                )),
                Some(Convergence::Converged)
            ),
            StopReason::Converged
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
