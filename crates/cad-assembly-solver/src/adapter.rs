//! [`AssemblyProblem`]/[`AdapterOutcome`]/[`AssemblySolverAdapter`] --
//! the numerical assembly-solver adapter contract (`AICAD-142`,
//! `project/OWNER_DECISIONS.md#D28`, `project/DECISION_LOG.md#DL-30`).
//!
//! `D28` requires a numerical backend to own algorithms only, never the
//! public meaning of a mate/joint. This module draws that boundary one
//! level stricter than `cad_constraints::sketch_constraint::SketchSolver`
//! (`DECISION_LOG.md#DL-20`'s own precedent, whose trait still takes the
//! semantic `Sketch`/`ConstraintSet` types directly): an
//! [`AssemblySolverAdapter`] never sees a `Mate`, `Joint`, `MateKind`, or
//! `OccurrenceTopologyRef` at all. It only ever sees an [`AssemblyProblem`]
//! -- a flat vector of free scalar unknowns and a list of opaque scalar
//! [`Residual`] functions -- and returns an [`AdapterOutcome`] over that
//! same numeric vocabulary. Lowering AICAD's own mate/joint semantics into
//! this numeric shape is `crate::grounding`'s job (`AICAD-143`), not this
//! module's and not any adapter's; an adapter that only ever receives
//! numbers cannot redefine what those numbers meant before lowering.
//!
//! # Why closures, not a symbolic expression enum
//!
//! A [`Residual`] is a boxed evaluator (`Fn(&[f64]) -> f64`), not a
//! symbolic AST a backend could pattern-match and special-case by relation
//! kind -- that would reopen exactly the "backend redefines relation
//! meaning" hole `D28` closes. An opaque `tag` travels alongside purely
//! for the *caller's* own diagnostic tracing (e.g. "which mate produced
//! this equation"); an adapter must never parse or branch on it.
//!
//! # `values`/`residuals`, not poses
//!
//! [`AdapterSolution::values`] is deliberately still a flat `f64` vector,
//! not a `WorldPose` -- turning solved numbers back into typed assembly
//! poses is `crate::grounding`'s own inverse of its own lowering (only the
//! lowering side knows which six consecutive scalars belonged to which
//! occurrence), never an adapter concern.

use std::fmt;
use std::rc::Rc;

/// A [`Residual`]'s own evaluator shape, named once so neither this
/// module nor `crate::grounding`'s residual-builder call sites need to
/// spell out the full `Rc<dyn Fn(&[f64]) -> f64>` type themselves.
pub type ResidualFn = Rc<dyn Fn(&[f64]) -> f64>;

/// One opaque scalar equation over an [`AssemblyProblem`]'s free variables
/// -- "opaque" meaning an adapter may evaluate it but must never inspect
/// `tag` to decide *how* to evaluate it (see module doc comment).
#[derive(Clone)]
pub struct Residual {
    tag: String,
    eval: ResidualFn,
}

impl fmt::Debug for Residual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Residual")
            .field("tag", &self.tag)
            .finish_non_exhaustive()
    }
}

impl Residual {
    pub fn new(tag: impl Into<String>, eval: ResidualFn) -> Residual {
        Residual {
            tag: tag.into(),
            eval,
        }
    }

    /// The caller-assigned opaque trace label -- never interpreted by an
    /// [`AssemblySolverAdapter`] (see module doc comment).
    pub fn tag(&self) -> &str {
        &self.tag
    }

    pub fn evaluate(&self, variables: &[f64]) -> f64 {
        (self.eval)(variables)
    }
}

/// Every way constructing an [`AssemblyProblem`] can fail -- shape errors
/// only, never anything about what the residuals mean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemError {
    /// `initial_guess.len() != variable_count`.
    InitialGuessLengthMismatch {
        variable_count: usize,
        guess_len: usize,
    },
    /// `initial_guess` contains a NaN/infinite value.
    NonFiniteInitialGuess { index: usize },
}

/// A semantic-neutral numeric problem: `variable_count` free scalar
/// unknowns, an `initial_guess` for them, and a list of scalar
/// [`Residual`] equations an adapter tries to drive toward zero. Nothing
/// here names a mate, joint, or occurrence (see module doc comment).
#[derive(Clone, Debug)]
pub struct AssemblyProblem {
    variable_count: usize,
    initial_guess: Vec<f64>,
    residuals: Vec<Residual>,
}

impl AssemblyProblem {
    pub fn new(
        variable_count: usize,
        initial_guess: Vec<f64>,
        residuals: Vec<Residual>,
    ) -> Result<AssemblyProblem, ProblemError> {
        if initial_guess.len() != variable_count {
            return Err(ProblemError::InitialGuessLengthMismatch {
                variable_count,
                guess_len: initial_guess.len(),
            });
        }
        if let Some(index) = initial_guess.iter().position(|v| !v.is_finite()) {
            return Err(ProblemError::NonFiniteInitialGuess { index });
        }
        Ok(AssemblyProblem {
            variable_count,
            initial_guess,
            residuals,
        })
    }

    pub fn variable_count(&self) -> usize {
        self.variable_count
    }

    pub fn initial_guess(&self) -> &[f64] {
        &self.initial_guess
    }

    pub fn residuals(&self) -> &[Residual] {
        &self.residuals
    }

    /// Evaluates every residual at `variables`, in declaration order.
    pub fn evaluate(&self, variables: &[f64]) -> Vec<f64> {
        self.residuals
            .iter()
            .map(|r| r.evaluate(variables))
            .collect()
    }
}

/// An adapter's numeric answer for one solve attempt: the variable
/// assignment it reached and the residual value of every equation there
/// (same order as [`AssemblyProblem::residuals`]).
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterSolution {
    pub values: Vec<f64>,
    pub residuals: Vec<f64>,
}

/// The adapter-boundary outcome vocabulary. Deliberately coarser than
/// `crate::grounding`'s own richer solved/underconstrained/overconstrained
/// classification (`AICAD-145`/`146`'s job): an adapter only ever reports
/// whether its own iteration converged, not what an unconverged or
/// residual-bearing result *means* for DOF/conflict purposes -- that
/// interpretation happens one layer up, over the same numeric evidence
/// every adapter returns here.
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterOutcome {
    /// Every residual fell within the adapter's own convergence tolerance.
    Converged {
        solution: AdapterSolution,
        iterations: u32,
    },
    /// The adapter's iteration budget was spent without every residual
    /// converging -- evidence, not a claim about *why* (could be
    /// inconsistent equations, could be a slow-converging but consistent
    /// system).
    NotConverged {
        solution: AdapterSolution,
        iterations: u32,
    },
    /// `problem` was structurally unusable (e.g. a residual referenced an
    /// out-of-range variable index and produced a NaN).
    InvalidInput { reason: String },
    /// The adapter's own bounded resource budget (iteration count, in the
    /// baseline backend) was exhausted -- distinct from `NotConverged` so
    /// a caller can tell "gave up early" apart from "used its full
    /// budget and still didn't converge" when an adapter chooses to draw
    /// that distinction; the baseline adapter (`AICAD-144`) folds both
    /// into `NotConverged` since it has exactly one bounded budget with no
    /// separate early-exit condition (see `crate::baseline`'s own doc
    /// comment).
    ResourceLimitExceeded {
        solution: AdapterSolution,
        iterations: u32,
    },
}

/// The `D28`/`DL-30` solver-independence boundary: a pluggable backend
/// that receives only [`AssemblyProblem`]'s numeric vocabulary and returns
/// only [`AdapterOutcome`]'s numeric vocabulary. See module doc comment.
pub trait AssemblySolverAdapter {
    fn solve(&self, problem: &AssemblyProblem) -> AdapterOutcome;
}

fn validate(problem: &AssemblyProblem) -> Option<AdapterOutcome> {
    if problem.initial_guess.iter().any(|v| !v.is_finite()) {
        return Some(AdapterOutcome::InvalidInput {
            reason: "initial guess contains a non-finite value".to_string(),
        });
    }
    let residuals = problem.evaluate(&problem.initial_guess);
    if residuals.iter().any(|r| !r.is_finite()) {
        return Some(AdapterOutcome::InvalidInput {
            reason: "a residual evaluated to a non-finite value at the initial guess".to_string(),
        });
    }
    None
}

/// The convergence tolerance every mock/reference adapter in this module
/// shares with `crate::baseline`'s own baseline backend
/// (`crate::baseline::CONVERGENCE_TOLERANCE`) -- kept as one named
/// constant here so "converged" means the same thing regardless of which
/// adapter answered.
pub const CONVERGENCE_TOLERANCE: f64 = 1e-9;

fn converged(residuals: &[f64]) -> bool {
    residuals.iter().all(|r| r.abs() <= CONVERGENCE_TOLERANCE)
}

/// A trivial reference adapter (`AICAD-142`'s own "mock/contract backend
/// sufficient to demonstrate backend independence", `AGENTS.md`'s
/// "SOLVER ADAPTER" section): performs no iteration at all and reports
/// whether the problem's own `initial_guess` already satisfies every
/// residual. Exists purely to exercise the [`AssemblySolverAdapter`]
/// contract end-to-end without any numerical algorithm.
#[derive(Debug, Clone, Copy, Default)]
pub struct EchoAdapter;

impl AssemblySolverAdapter for EchoAdapter {
    fn solve(&self, problem: &AssemblyProblem) -> AdapterOutcome {
        if let Some(invalid) = validate(problem) {
            return invalid;
        }
        let values = problem.initial_guess().to_vec();
        let residuals = problem.evaluate(&values);
        let solution = AdapterSolution { values, residuals };
        if converged(&solution.residuals) {
            AdapterOutcome::Converged {
                solution,
                iterations: 0,
            }
        } else {
            AdapterOutcome::NotConverged {
                solution,
                iterations: 0,
            }
        }
    }
}

/// A second, independently-implemented reference adapter -- starts every
/// variable at `0.0` instead of `problem.initial_guess()`, then reports
/// the same way [`EchoAdapter`] does. Used only to prove backend
/// substitutability (`AICAD-142`'s "alternate-adapter mapping fixture"
/// required check): on a problem whose residuals are identically zero
/// everywhere (the only case two zero-iteration adapters can agree on
/// regardless of starting point), both adapters must report the same
/// [`AdapterOutcome`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ZeroStartAdapter;

impl AssemblySolverAdapter for ZeroStartAdapter {
    fn solve(&self, problem: &AssemblyProblem) -> AdapterOutcome {
        if let Some(invalid) = validate(problem) {
            return invalid;
        }
        let values = vec![0.0; problem.variable_count()];
        let residuals = problem.evaluate(&values);
        let solution = AdapterSolution { values, residuals };
        if converged(&solution.residuals) {
            AdapterOutcome::Converged {
                solution,
                iterations: 0,
            }
        } else {
            AdapterOutcome::NotConverged {
                solution,
                iterations: 0,
            }
        }
    }
}

/// An adapter that always reports [`AdapterOutcome::InvalidInput`] --
/// `AICAD-142`'s own "structured failure tests" required check exercised
/// as a distinct backend, proving a conforming adapter may reject any
/// input it chooses without changing the trait's own contract shape.
#[derive(Debug, Clone, Copy, Default)]
pub struct RejectingAdapter;

impl AssemblySolverAdapter for RejectingAdapter {
    fn solve(&self, _problem: &AssemblyProblem) -> AdapterOutcome {
        AdapterOutcome::InvalidInput {
            reason: "RejectingAdapter rejects every problem unconditionally".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constant_residual(value: f64) -> Residual {
        Residual::new("const", Rc::new(move |_vars: &[f64]| value))
    }

    fn variable_residual(index: usize) -> Residual {
        Residual::new(
            format!("var:{index}"),
            Rc::new(move |vars: &[f64]| vars[index]),
        )
    }

    #[test]
    fn mismatched_initial_guess_length_is_rejected_at_construction() {
        let err = AssemblyProblem::new(2, vec![0.0], vec![]).unwrap_err();
        assert_eq!(
            err,
            ProblemError::InitialGuessLengthMismatch {
                variable_count: 2,
                guess_len: 1,
            }
        );
    }

    #[test]
    fn non_finite_initial_guess_is_rejected_at_construction() {
        let err = AssemblyProblem::new(1, vec![f64::NAN], vec![]).unwrap_err();
        assert_eq!(err, ProblemError::NonFiniteInitialGuess { index: 0 });
    }

    #[test]
    fn echo_adapter_reports_converged_when_the_initial_guess_already_satisfies_every_residual() {
        let problem = AssemblyProblem::new(1, vec![0.0], vec![variable_residual(0)]).unwrap();
        let outcome = EchoAdapter.solve(&problem);
        match outcome {
            AdapterOutcome::Converged {
                solution,
                iterations,
            } => {
                assert_eq!(iterations, 0);
                assert_eq!(solution.values, vec![0.0]);
                assert_eq!(solution.residuals, vec![0.0]);
            }
            other => panic!("expected Converged, got {other:?}"),
        }
    }

    #[test]
    fn echo_adapter_reports_not_converged_when_the_initial_guess_violates_a_residual() {
        let problem = AssemblyProblem::new(1, vec![1.0], vec![variable_residual(0)]).unwrap();
        let outcome = EchoAdapter.solve(&problem);
        assert!(matches!(outcome, AdapterOutcome::NotConverged { .. }));
    }

    #[test]
    fn rejecting_adapter_always_reports_invalid_input() {
        let problem = AssemblyProblem::new(0, vec![], vec![]).unwrap();
        let outcome = RejectingAdapter.solve(&problem);
        assert!(matches!(outcome, AdapterOutcome::InvalidInput { .. }));
    }

    #[test]
    fn two_independently_implemented_adapters_agree_on_an_already_solved_problem() {
        // Every residual is identically zero everywhere -- the only case
        // both a starting-at-`initial_guess` and a starting-at-`0.0`
        // zero-iteration adapter can agree on regardless of what
        // `initial_guess` itself was.
        let problem =
            AssemblyProblem::new(3, vec![7.0, -2.0, 0.5], vec![constant_residual(0.0)]).unwrap();
        let echo = EchoAdapter.solve(&problem);
        let zero_start = ZeroStartAdapter.solve(&problem);
        match (echo, zero_start) {
            (
                AdapterOutcome::Converged { solution: a, .. },
                AdapterOutcome::Converged { solution: b, .. },
            ) => {
                assert_eq!(
                    a.residuals, b.residuals,
                    "both agree the relation is satisfied"
                );
            }
            other => panic!("expected both adapters to converge, got {other:?}"),
        }
    }

    #[test]
    fn a_residual_producing_a_non_finite_value_at_the_initial_guess_is_invalid_input() {
        let bad = Residual::new("div", Rc::new(|vars: &[f64]| 1.0 / vars[0]));
        let problem = AssemblyProblem::new(1, vec![0.0], vec![bad]).unwrap();
        let outcome = EchoAdapter.solve(&problem);
        assert!(matches!(outcome, AdapterOutcome::InvalidInput { .. }));
    }
}
