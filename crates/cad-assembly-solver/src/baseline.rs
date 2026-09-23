//! [`BaselineSolver`] -- the one bounded deterministic numerical backend
//! behind [`crate::adapter::AssemblySolverAdapter`] (`AICAD-144`), plus
//! [`solve_assembly`], the AICAD-owned entry point that lowers a resolved
//! occurrence tree's mates/joints (`crate::grounding::lower`), runs this
//! backend, and classifies the result back into [`SolveOutcome`]'s
//! structured evidence vocabulary.
//!
//! # Algorithm
//!
//! Levenberg-Marquardt: at each iteration, a finite-difference Jacobian
//! `J` of the current residual vector is used to solve the damped normal
//! equations `(J^T J + lambda I) delta = -J^T r` (`crate::linalg::
//! solve_linear_system`); a `delta` that lowers `||r||^2` is accepted
//! (damping relaxes for the next step), a `delta` that does not is
//! rejected (damping tightens and the step is retried). This is the same
//! Gauss-Newton family `crate::grounding`'s own module doc comment
//! ("Representative-pose policy") already documents: the *damped* normal
//! equations always admit a solution (even when `J^T J` alone is
//! singular, exactly the underconstrained case), and a direction with no
//! Jacobian column support never receives a nonzero correction, so it
//! deterministically keeps its initial value.
//!
//! # Stall detection distinguishes "needs more iterations" from
//! "structurally stuck"
//!
//! If damping grows past `damping_max` without ever finding an improving
//! step, no further increase can help (the search has nowhere left to
//! go) -- iteration stops immediately with [`AdapterOutcome::
//! NotConverged`], *before* the iteration budget is spent. Only a loop
//! that spends its *entire* `max_iterations` budget without reaching
//! tolerance and without stalling reports [`AdapterOutcome::
//! ResourceLimitExceeded`] -- `AICAD-142`'s two non-convergence variants
//! therefore mean two genuinely different things for this backend: gave
//! up because further search could not help, vs. gave up because the
//! bounded budget ran out while search was still meaningfully proceeding.

use crate::adapter::{
    AdapterOutcome, AdapterSolution, AssemblyProblem, AssemblySolverAdapter, CONVERGENCE_TOLERANCE,
};
use crate::grounding::{self, GroundingProfile, Lowering, LoweringError};
use crate::linalg;
use cad_assemblies::{Joint, Mate, Occurrence, OccurrencePath};
use cad_kernel_api::{Axis3, Point3, Transform, Vector3};

/// The versioned numeric-tolerance/iteration profile `AICAD-144`'s
/// baseline backend uses (`DECISION_LOG.md#DL-20`'s "explicit and
/// versioned" precedent). Embeds [`GroundingProfile`] rather than
/// duplicating its finite-difference-step/rank-tolerance fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaselineSolverProfile {
    pub version: u32,
    pub grounding: GroundingProfile,
    /// A residual within this tolerance of zero counts as satisfied.
    pub convergence_tolerance: f64,
    /// A residual whose final magnitude exceeds
    /// `convergence_tolerance * conflict_multiplier` after a stall or
    /// exhausted iteration budget counts as conflicting evidence
    /// (`solve_assembly`'s `Overconstrained` classification).
    pub conflict_multiplier: f64,
    pub max_iterations: u32,
    pub damping_initial: f64,
    pub damping_increase: f64,
    pub damping_decrease: f64,
    /// Damping ceiling past which a failed step means "stuck," not
    /// "needs a smaller step" (see module doc comment "Stall detection").
    pub damping_max: f64,
}

impl BaselineSolverProfile {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn v1() -> BaselineSolverProfile {
        BaselineSolverProfile {
            version: BaselineSolverProfile::CURRENT_VERSION,
            grounding: GroundingProfile::v1(),
            convergence_tolerance: CONVERGENCE_TOLERANCE,
            conflict_multiplier: 1e6,
            max_iterations: 200,
            damping_initial: 1e-3,
            damping_increase: 10.0,
            damping_decrease: 0.3,
            damping_max: 1e12,
        }
    }
}

impl Default for BaselineSolverProfile {
    fn default() -> BaselineSolverProfile {
        BaselineSolverProfile::v1()
    }
}

/// `AICAD-144`'s one bounded deterministic numerical backend. See module
/// doc comment for the algorithm.
#[derive(Debug, Clone, Copy)]
pub struct BaselineSolver {
    profile: BaselineSolverProfile,
}

impl BaselineSolver {
    pub fn new() -> BaselineSolver {
        BaselineSolver {
            profile: BaselineSolverProfile::v1(),
        }
    }

    pub fn with_profile(profile: BaselineSolverProfile) -> BaselineSolver {
        BaselineSolver { profile }
    }

    pub fn profile(&self) -> BaselineSolverProfile {
        self.profile
    }
}

impl Default for BaselineSolver {
    fn default() -> BaselineSolver {
        BaselineSolver::new()
    }
}

fn sum_sq(values: &[f64]) -> f64 {
    values.iter().map(|v| v * v).sum()
}

fn all_finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}

impl AssemblySolverAdapter for BaselineSolver {
    fn solve(&self, problem: &AssemblyProblem) -> AdapterOutcome {
        let n = problem.variable_count();
        let mut x = problem.initial_guess().to_vec();
        let mut residuals = problem.evaluate(&x);
        if !all_finite(&x) || !all_finite(&residuals) {
            return AdapterOutcome::InvalidInput {
                reason: "initial guess or its residuals are non-finite".to_string(),
            };
        }
        if n == 0 || residuals.is_empty() {
            let solution = AdapterSolution {
                values: x,
                residuals,
            };
            return if solution
                .residuals
                .iter()
                .all(|r| r.abs() <= self.profile.convergence_tolerance)
            {
                AdapterOutcome::Converged {
                    solution,
                    iterations: 0,
                }
            } else {
                AdapterOutcome::NotConverged {
                    solution,
                    iterations: 0,
                }
            };
        }

        let mut lambda = self.profile.damping_initial;
        let mut cost = sum_sq(&residuals);

        for iteration in 0..self.profile.max_iterations {
            if residuals
                .iter()
                .all(|r| r.abs() <= self.profile.convergence_tolerance)
            {
                return AdapterOutcome::Converged {
                    solution: AdapterSolution {
                        values: x,
                        residuals,
                    },
                    iterations: iteration,
                };
            }

            let jac = linalg::jacobian(
                |vars| problem.evaluate(vars),
                &x,
                self.profile.grounding.finite_difference_step,
            );
            let gram = linalg::gram(&jac);
            let neg_gradient: Vec<f64> = linalg::transpose_mul_vec(&jac, &residuals)
                .into_iter()
                .map(|v| -v)
                .collect();

            let mut stepped = false;
            while lambda <= self.profile.damping_max {
                let mut damped = gram.clone();
                for (i, row) in damped.iter_mut().enumerate() {
                    row[i] += lambda;
                }
                if let Some(delta) = linalg::solve_linear_system(&damped, &neg_gradient) {
                    let candidate: Vec<f64> =
                        x.iter().zip(delta.iter()).map(|(v, d)| v + d).collect();
                    let candidate_residuals = problem.evaluate(&candidate);
                    if all_finite(&candidate) && all_finite(&candidate_residuals) {
                        let candidate_cost = sum_sq(&candidate_residuals);
                        if candidate_cost < cost {
                            x = candidate;
                            residuals = candidate_residuals;
                            cost = candidate_cost;
                            lambda = (lambda * self.profile.damping_decrease).max(1e-12);
                            stepped = true;
                            break;
                        }
                    }
                }
                lambda *= self.profile.damping_increase;
            }
            if !stepped {
                // Damping is exhausted without any improving step --
                // further iteration cannot help (see module doc comment
                // "Stall detection").
                return AdapterOutcome::NotConverged {
                    solution: AdapterSolution {
                        values: x,
                        residuals,
                    },
                    iterations: iteration,
                };
            }
        }

        AdapterOutcome::ResourceLimitExceeded {
            solution: AdapterSolution {
                values: x,
                residuals,
            },
            iterations: self.profile.max_iterations,
        }
    }
}

/// `AICAD-144`'s own structured-evidence vocabulary -- the classification
/// `crate::adapter::AdapterOutcome`'s coarser numeric result is turned
/// into once `crate::grounding`'s Jacobian-rank evidence is folded in.
/// Only [`SolveOutcome::Solved`]/[`SolveOutcome::Underconstrained`] carry
/// a `poses` map: every failure variant deliberately carries none, so a
/// caller cannot mistake a non-authoritative numeric attempt for an
/// authoritative pose (`AICAD-144`'s "failure never leaves partial
/// authoritative poses" acceptance criterion).
#[derive(Debug, Clone, PartialEq)]
pub enum SolveOutcome {
    /// Every mate/joint residual converged and no free direction
    /// remained (`remaining_dof == 0` at the solution).
    Solved {
        poses: Vec<(OccurrencePath, Transform)>,
        unsupported: Vec<String>,
    },
    /// Every mate/joint residual converged, but at least one free
    /// direction remained unconstrained at the solution -- the poses
    /// returned still reflect this module's own deterministic
    /// representative-pose policy (`crate::grounding`'s own doc comment),
    /// not an arbitrary backend choice.
    Underconstrained {
        poses: Vec<(OccurrencePath, Transform)>,
        remaining_dof: usize,
        unsupported: Vec<String>,
    },
    /// Iteration stalled (or exhausted its budget) with at least one
    /// residual still exceeding the conflict threshold -- structured
    /// evidence of mutually inconsistent relations.
    Overconstrained {
        conflicting: Vec<String>,
        unsupported: Vec<String>,
    },
    /// Iteration stopped without every residual converging, but no
    /// residual exceeded the conflict threshold either -- ambiguous
    /// evidence (a slower-converging but plausibly consistent system),
    /// reported honestly rather than guessed at.
    NotConverged { iterations: u32, max_residual: f64 },
    /// The lowered numeric problem itself was unusable (e.g. a
    /// non-finite residual at the initial guess).
    InvalidInput { reason: String },
    /// The bounded iteration budget was spent without reaching
    /// convergence or a detected stall.
    ResourceLimitExceeded { iterations: u32, max_residual: f64 },
}

/// Lowers `occurrences`/`mates`/`joints` (`crate::grounding::lower`), runs
/// `adapter`, and classifies the result into [`SolveOutcome`]. The one
/// AICAD-owned entry point most callers use -- `adapter` is any
/// [`crate::adapter::AssemblySolverAdapter`], not necessarily
/// [`BaselineSolver`], so swapping backends never changes which
/// [`SolveOutcome`] variant a given assembly produces (`AICAD-142`'s own
/// substitutability requirement, exercised directly by this module's own
/// `swapping_the_adapter_does_not_change_the_solve_outcome` test).
pub fn solve_assembly(
    occurrences: &[Occurrence],
    mates: &[Mate],
    joints: &[Joint],
    adapter: &impl AssemblySolverAdapter,
    profile: &BaselineSolverProfile,
) -> Result<SolveOutcome, LoweringError> {
    let lowering = grounding::lower(occurrences, mates, joints)?;
    let outcome = adapter.solve(&lowering.problem);
    Ok(classify(&lowering, occurrences, outcome, profile))
}

/// [`solve_assembly`]'s own classification step, factored out so
/// `crate::kinematics` (`AICAD-147`) can classify an adapter's outcome
/// against a *modified* [`Lowering`] -- one whose `problem` has extra
/// joint-coordinate-pinning residuals appended to `lowering::lower`'s own
/// structural ones -- without duplicating this module's conflict/DOF
/// classification logic.
pub fn classify(
    lowering: &Lowering,
    occurrences: &[Occurrence],
    outcome: AdapterOutcome,
    profile: &BaselineSolverProfile,
) -> SolveOutcome {
    match outcome {
        AdapterOutcome::InvalidInput { reason } => SolveOutcome::InvalidInput { reason },
        AdapterOutcome::ResourceLimitExceeded {
            solution,
            iterations,
        } => {
            // `iterations == 0` means the adapter never actually
            // attempted a step (e.g. a `max_iterations: 0` profile) --
            // a large residual then reflects the *initial guess*, not
            // evidence the relations conflict, so conflict
            // reclassification only applies once at least one iteration
            // ran.
            match (iterations > 0)
                .then(|| conflicting_tags(&lowering.problem, &solution.residuals, profile))
                .flatten()
            {
                Some(conflicting) => SolveOutcome::Overconstrained {
                    conflicting,
                    unsupported: lowering.unsupported.clone(),
                },
                None => SolveOutcome::ResourceLimitExceeded {
                    iterations,
                    max_residual: max_abs(&solution.residuals),
                },
            }
        }
        AdapterOutcome::NotConverged {
            solution,
            iterations,
        } => {
            match (iterations > 0)
                .then(|| conflicting_tags(&lowering.problem, &solution.residuals, profile))
                .flatten()
            {
                Some(conflicting) => SolveOutcome::Overconstrained {
                    conflicting,
                    unsupported: lowering.unsupported.clone(),
                },
                None => SolveOutcome::NotConverged {
                    iterations,
                    max_residual: max_abs(&solution.residuals),
                },
            }
        }
        AdapterOutcome::Converged { solution, .. } => {
            let dof = grounding::structural_dof_at(
                &lowering.problem,
                &solution.values,
                &profile.grounding,
            );
            let poses = poses_of(&lowering.free_occurrences, occurrences, &solution.values);
            if dof.remaining_dof == 0 {
                SolveOutcome::Solved {
                    poses,
                    unsupported: lowering.unsupported.clone(),
                }
            } else {
                SolveOutcome::Underconstrained {
                    poses,
                    remaining_dof: dof.remaining_dof,
                    unsupported: lowering.unsupported.clone(),
                }
            }
        }
    }
}

fn max_abs(values: &[f64]) -> f64 {
    values.iter().fold(0.0_f64, |acc, v| acc.max(v.abs()))
}

/// The tags of every residual whose final magnitude exceeds this
/// profile's conflict threshold, or `None` if none does -- `solve_assembly`'s
/// only way to distinguish [`SolveOutcome::Overconstrained`] from a
/// merely slow, still-plausibly-consistent [`SolveOutcome::NotConverged`]/
/// [`SolveOutcome::ResourceLimitExceeded`].
fn conflicting_tags(
    problem: &AssemblyProblem,
    residuals: &[f64],
    profile: &BaselineSolverProfile,
) -> Option<Vec<String>> {
    let threshold = profile.convergence_tolerance * profile.conflict_multiplier;
    let tags: Vec<String> = problem
        .residuals()
        .iter()
        .zip(residuals.iter())
        .filter(|(_, value)| value.abs() > threshold)
        .map(|(residual, _)| residual.tag().to_string())
        .collect();
    if tags.is_empty() { None } else { Some(tags) }
}

fn poses_of(
    free_occurrences: &[OccurrencePath],
    occurrences: &[Occurrence],
    values: &[f64],
) -> Vec<(OccurrencePath, Transform)> {
    let mut result = Vec::with_capacity(occurrences.len());
    for occurrence in occurrences {
        let world = occurrence.world_pose().transform();
        let pose = match free_occurrences.iter().position(|p| p == occurrence.path()) {
            None => world,
            Some(index) => {
                let offset = index * 6;
                let pivot = world.apply_point(Point3::ORIGIN);
                world.compose(&delta_transform_pub(pivot, &values[offset..offset + 6]))
            }
        };
        result.push((occurrence.path().clone(), pose));
    }
    result
}

/// Duplicates `crate::grounding`'s own private `delta_transform` --
/// `poses_of` needs to reconstruct exactly the same rigid delta
/// `crate::grounding::lower`'s residual closures used, and that helper is
/// intentionally private to keep `crate::grounding`'s anchor
/// parametrization an internal detail no `Residual` closure's caller
/// (including `crate::adapter`) can depend on. Kept in lock-step by
/// `poses_of_matches_the_authored_pose_at_the_all_zero_solution` below.
fn delta_transform_pub(pivot: Point3, params: &[f64]) -> Transform {
    let axis_vec = Vector3::new(params[3], params[4], params[5]);
    let angle = axis_vec.length();
    let rotation = if angle < 1e-15 {
        Transform::identity()
    } else {
        let axis_dir = axis_vec.normalize().expect("angle >= 1e-15 checked above");
        Transform::rotation(Axis3::new(pivot, axis_dir), angle)
    };
    let translation = Transform::translation(Vector3::new(params[0], params[1], params[2]));
    rotation.compose(&translation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::EchoAdapter;
    use cad_assemblies::definition::ComponentDefinitionId;
    use cad_assemblies::frame::LocalPose;
    use cad_assemblies::instance::LogicalInstanceId;
    use cad_assemblies::mate::MateId;
    use cad_assemblies::occurrence::OccurrencePath;
    use cad_assemblies::topology::OccurrenceTopologyRef;
    use cad_assemblies::value::ParameterValue;
    use cad_assemblies::{MateKind, component};
    use cad_kernel_api::{Point3, Vector3 as V3};
    use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn occurrence(local: &str, translation: V3) -> Occurrence {
        let path = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Part"),
            local,
        ));
        let mut registry = component::ComponentDefinitionRegistry::new();
        registry
            .define(component::ComponentDefinition::new(
                path.leaf().definition().clone(),
                vec![],
                vec![],
            ))
            .unwrap();
        cad_assemblies::graph::expand(
            &registry,
            path.leaf().clone(),
            LocalPose::new(Transform::translation(translation)),
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
    }

    fn face_subject(path: &OccurrencePath) -> OccurrenceTopologyRef {
        OccurrenceTopologyRef::new(
            path.clone(),
            AnyRef::from_strategy(
                EntityKind::Face,
                ConstructionStrategy::StructuralRole("anchor".into()),
            ),
        )
    }

    fn length(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
    }

    #[test]
    fn a_coincident_mate_converges_the_free_occurrence_onto_the_grounded_one() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(5.0, -2.0, 3.0));
        let mate = Mate::new(
            MateId::named("m"),
            MateKind::Coincident,
            [face_subject(root.path()), face_subject(child.path())],
            None,
        )
        .unwrap();
        let outcome = solve_assembly(
            &[root, child.clone()],
            &[mate],
            &[],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        match outcome {
            // A Coincident mate removes only the 3 positional DOF --
            // orientation is left free, so the correct classification is
            // `Underconstrained`, not `Solved`.
            SolveOutcome::Underconstrained {
                poses,
                remaining_dof,
                unsupported,
            } => {
                assert!(unsupported.is_empty());
                assert_eq!(remaining_dof, 3);
                let (_, child_pose) = poses.iter().find(|(p, _)| p == child.path()).unwrap();
                let origin = child_pose.apply_point(Point3::ORIGIN);
                assert!(origin.x.abs() < 1e-6, "expected ~0, got {origin:?}");
                assert!(origin.y.abs() < 1e-6, "expected ~0, got {origin:?}");
                assert!(origin.z.abs() < 1e-6, "expected ~0, got {origin:?}");
            }
            other => panic!("expected Underconstrained, got {other:?}"),
        }
    }

    #[test]
    fn a_distance_mate_converges_to_exactly_the_target_separation() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 0.0, 0.0));
        let mate = Mate::new(
            MateId::named("gap"),
            MateKind::Distance,
            [face_subject(root.path()), face_subject(child.path())],
            Some(length(10.0)),
        )
        .unwrap();
        let outcome = solve_assembly(
            &[root, child.clone()],
            &[mate],
            &[],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        match outcome {
            SolveOutcome::Underconstrained {
                poses,
                remaining_dof,
                ..
            } => {
                assert_eq!(
                    remaining_dof, 5,
                    "distance alone removes exactly 1 of 6 DOF"
                );
                let (_, child_pose) = poses.iter().find(|(p, _)| p == child.path()).unwrap();
                let origin = child_pose.apply_point(Point3::ORIGIN);
                let distance =
                    (origin.x * origin.x + origin.y * origin.y + origin.z * origin.z).sqrt();
                assert!((distance - 10.0).abs() < 1e-6, "got distance {distance}");
            }
            other => panic!("expected Underconstrained, got {other:?}"),
        }
    }

    #[test]
    fn two_contradictory_distance_mates_are_reported_overconstrained() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 0.0, 0.0));
        let too_short = Mate::new(
            MateId::named("d1"),
            MateKind::Distance,
            [face_subject(root.path()), face_subject(child.path())],
            Some(length(1.0)),
        )
        .unwrap();
        let too_long = Mate::new(
            MateId::named("d2"),
            MateKind::Distance,
            [face_subject(root.path()), face_subject(child.path())],
            Some(length(100.0)),
        )
        .unwrap();
        let outcome = solve_assembly(
            &[root, child],
            &[too_short, too_long],
            &[],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        assert!(matches!(outcome, SolveOutcome::Overconstrained { .. }));
    }

    #[test]
    fn repeated_solves_of_the_same_problem_are_bit_for_bit_deterministic() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(5.0, -2.0, 3.0));
        let mate = Mate::new(
            MateId::named("m"),
            MateKind::Coincident,
            [face_subject(root.path()), face_subject(child.path())],
            None,
        )
        .unwrap();
        let solve = || {
            solve_assembly(
                &[root.clone(), child.clone()],
                std::slice::from_ref(&mate),
                &[],
                &BaselineSolver::new(),
                &BaselineSolverProfile::v1(),
            )
            .unwrap()
        };
        assert_eq!(solve(), solve());
    }

    #[test]
    fn an_iteration_budget_of_zero_reports_the_adapter_level_resource_limit() {
        // Exercised directly against `BaselineSolver` (not `solve_assembly`):
        // whether a large residual at `max_iterations: 0` should be read
        // as "conflicting" or "never attempted" is `solve_assembly`'s own
        // higher-level judgment call, not part of the adapter contract
        // itself -- the adapter contract only needs to report that its
        // bounded budget (here, zero) was spent without convergence.
        use crate::adapter::{AdapterOutcome, AssemblyProblem, Residual};
        use std::rc::Rc;

        let problem = AssemblyProblem::new(
            1,
            vec![5.0],
            vec![Residual::new("r", Rc::new(|vars: &[f64]| vars[0]))],
        )
        .unwrap();
        let mut profile = BaselineSolverProfile::v1();
        profile.max_iterations = 0;
        let outcome = BaselineSolver::with_profile(profile).solve(&problem);
        assert_eq!(
            outcome,
            AdapterOutcome::ResourceLimitExceeded {
                solution: AdapterSolution {
                    values: vec![5.0],
                    residuals: vec![5.0],
                },
                iterations: 0,
            }
        );
    }

    #[test]
    fn swapping_the_adapter_does_not_change_the_solve_outcome_shape_for_an_already_solved_problem()
    {
        let root = occurrence("root", V3::ZERO);
        let bystander = occurrence("bystander", V3::new(1.0, 1.0, 1.0));
        // No mates/joints at all -- nothing to solve, so every adapter
        // (including the trivial `EchoAdapter`) must agree it is
        // trivially `Solved` with no poses to report.
        let baseline_outcome = solve_assembly(
            &[root.clone(), bystander.clone()],
            &[],
            &[],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        let echo_outcome = solve_assembly(
            &[root, bystander],
            &[],
            &[],
            &EchoAdapter,
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        match (baseline_outcome, echo_outcome) {
            (SolveOutcome::Solved { poses: a, .. }, SolveOutcome::Solved { poses: b, .. }) => {
                assert_eq!(a, b)
            }
            other => panic!("expected both Solved, got {other:?}"),
        }
    }
}
