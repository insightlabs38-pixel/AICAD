//! Deterministic grounding, initial pose, and lowering of AICAD-owned
//! mate/joint relations into `crate::adapter::AssemblyProblem`
//! (`AICAD-143`, `project/DECISION_LOG.md#DL-30`).
//!
//! `DL-30` requires an explicit grounding/root policy, an explicit
//! initial-pose policy, and a deterministic representative-pose rule for
//! underconstrained systems -- all fixed *before* `crate::baseline`'s
//! iterative solver exists, so none of them becomes an accidental
//! by-product of whichever numerical algorithm ships first.
//!
//! # Grounding policy
//!
//! Stage 6's assembly IR (`cad_assemblies::graph::expand`) always
//! resolves from exactly one top-level occurrence (`OccurrencePath::depth()
//! == 1`). Stage-6 source syntax has no way yet to declare an alternate
//! explicit ground, so this module's policy is the other half of `DL-30`'s
//! "use an explicit root when the design supplies one; otherwise choose a
//! deterministic canonical representative": **the assembly's own top-level
//! occurrence is always the canonical ground.** Its `WorldPose` is never a
//! free variable, regardless of how many mates/joints name it as a
//! subject. A later task adding explicit ground-declaration syntax
//! extends this policy (the "supplies one" branch); it does not replace
//! it.
//!
//! # Free-variable policy
//!
//! Only occurrences that actually appear as a mate subject or joint
//! parent/child, other than the ground, become free rigid-pose unknowns.
//! An occurrence no relation ever names keeps its authored `WorldPose`
//! exactly -- `AGENTS.md`'s "An underconstrained assembly must not
//! silently acquire a backend-dependent arbitrary pose" extended to mean
//! "an *unconstrained* occurrence must not move at all," not just "must
//! not move arbitrarily." The free set is a `BTreeSet<OccurrencePath>`
//! (deterministic order, independent of `mates`/`joints` input order --
//! see `relation_order_does_not_change_the_free_variable_set` below).
//!
//! # Pose parametrization and the representative-pose policy
//!
//! Each free occurrence contributes six consecutive scalars: a
//! translation `(dx, dy, dz)` and an axis-angle rotation vector
//! `(rx, ry, rz)` (angle = magnitude, axis = direction), applied as a
//! rigid delta *pivoted at that occurrence's own authored world origin*
//! (see `Anchor::transform`) on top of its authored `WorldPose` -- so the
//! all-zero vector reproduces the design's own authored pose exactly,
//! satisfying `AICAD-143`'s "initial guesses do not arbitrarily change
//! source-visible pose semantics." `crate::baseline`'s Gauss-Newton/
//! Levenberg-Marquardt iteration takes the minimum-norm correction its own
//! normal equations admit at every step; a direction the Jacobian leaves
//! completely unconstrained therefore never moves away from `0.0` (the
//! authored pose). **This is `AICAD-143`'s own representative-pose
//! policy, not an incidental property of whichever algorithm
//! `AICAD-144` ships**: an underconstrained assembly's free directions
//! deterministically keep their authored value rather than drifting to an
//! arbitrary backend-dependent pose.
//!
//! # Anchor geometry (disclosed scope bound)
//!
//! A mate/joint subject's geometric meaning here is its own occurrence's
//! frame -- `point = world origin`, `primary = world +Z`, `secondary =
//! world +X` -- **not** the literal referenced sub-entity's real kernel
//! geometry (no kernel/OCCT dependency exists anywhere in this crate).
//! This is a disclosed baseline-solver scope bound, not silent
//! misbehavior: wiring a mate/joint subject to its own declared
//! `crate::interface`-style anchor frame is later work once mechanical
//! interfaces are threaded into solving, not this batch's job. One
//! consequence: `MateKind::Tangent` has no meaningful realization at this
//! anchor-frame granularity (it needs curvature/radius data this
//! abstraction does not carry) and is reported `unsupported` rather than
//! silently solved incorrectly -- mirroring `cad_constraints::
//! sketch_solver`'s own honest `SolveStatus::Unsupported` precedent.

use crate::adapter::{AssemblyProblem, ProblemError, Residual};
use crate::linalg;
use cad_assemblies::{Joint, JointKind, Mate, MateKind, Occurrence, OccurrencePath};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Severity, SeverityLetter};
use cad_kernel_api::{Axis3, Direction3, Point3, Transform, Vector3};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The versioned numeric-tolerance profile every solver-adapter-crate
/// numeric analysis shares (`DECISION_LOG.md#DL-20`'s "numerical
/// tolerances... must be explicit and versioned" precedent, mirroring
/// `cad_constraints::sketch_solver::SketchSolverProfile`). `crate::
/// baseline::BaselineSolverProfile` extends this with its own
/// iteration-specific constants rather than duplicating these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundingProfile {
    pub version: u32,
    /// Central-finite-difference step used by every Jacobian this crate
    /// computes.
    pub finite_difference_step: f64,
    /// Absolute pivot-magnitude threshold `crate::linalg::rank` treats as
    /// nonzero. A fixed absolute (not scale-relative) tolerance tuned for
    /// typical mm-to-m--scale assemblies -- a disclosed limitation for
    /// assemblies at extreme physical scale, matching `cad_validation::
    /// profile::ComparisonProfile`'s own fixed-absolute-tolerance
    /// precedent.
    pub rank_tolerance: f64,
}

impl GroundingProfile {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn v1() -> GroundingProfile {
        GroundingProfile {
            version: GroundingProfile::CURRENT_VERSION,
            finite_difference_step: 1e-6,
            rank_tolerance: 1e-6,
        }
    }
}

impl Default for GroundingProfile {
    fn default() -> GroundingProfile {
        GroundingProfile::v1()
    }
}

/// One free occurrence's rigid-pose anchor: its authored `Transform`, the
/// world-space pivot a delta rotates about, and its own six-scalar
/// variable offset (`None` for the grounded root or any non-participating
/// occurrence, which are fixed constants instead).
#[derive(Debug, Clone, Copy)]
struct Anchor {
    initial: Transform,
    pivot: Point3,
    offset: Option<usize>,
}

impl Anchor {
    fn fixed(world: Transform) -> Anchor {
        Anchor {
            initial: world,
            pivot: world.apply_point(Point3::ORIGIN),
            offset: None,
        }
    }

    fn free(world: Transform, offset: usize) -> Anchor {
        Anchor {
            initial: world,
            pivot: world.apply_point(Point3::ORIGIN),
            offset: Some(offset),
        }
    }

    /// The occurrence's world transform at `vars`: its authored pose,
    /// exactly, for a fixed anchor; its authored pose composed with a
    /// pivot-at-its-own-origin rigid delta for a free one (see module doc
    /// comment "Pose parametrization").
    fn transform(&self, vars: &[f64]) -> Transform {
        match self.offset {
            None => self.initial,
            Some(off) => self
                .initial
                .compose(&delta_transform(self.pivot, &vars[off..off + 6])),
        }
    }

    fn point(&self, vars: &[f64]) -> Point3 {
        self.transform(vars).apply_point(Point3::ORIGIN)
    }

    fn primary(&self, vars: &[f64]) -> Direction3 {
        self.transform(vars).apply_direction(Direction3::Z)
    }

    fn secondary(&self, vars: &[f64]) -> Direction3 {
        self.transform(vars).apply_direction(Direction3::X)
    }
}

fn delta_transform(pivot: Point3, params: &[f64]) -> Transform {
    debug_assert_eq!(params.len(), 6);
    let axis_vec = Vector3::new(params[3], params[4], params[5]);
    let angle = axis_vec.length();
    let rotation = if angle < 1e-15 {
        Transform::identity()
    } else {
        let axis_dir = axis_vec
            .normalize()
            .expect("angle >= 1e-15 checked above, so axis_vec is not near-zero");
        Transform::rotation(Axis3::new(pivot, axis_dir), angle)
    };
    let translation = Transform::translation(Vector3::new(params[0], params[1], params[2]));
    rotation.compose(&translation)
}

/// Every way [`lower`] can fail closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoweringError {
    /// A mate/joint subject named an [`OccurrencePath`] absent from the
    /// given occurrence list -- fail closed, mirroring
    /// `cad_assemblies::reference::AssemblyBrokenReason::OccurrenceNotFound`'s
    /// own precedent for the same underlying mistake at the reference
    /// layer.
    UnknownOccurrence(OccurrencePath),
    /// `occurrences` contained no depth-1 (top-level/root) entry --
    /// `cad_assemblies::graph::expand` always produces exactly one, so
    /// this only happens for a hand-built (non-`expand`-produced) input.
    NoRootOccurrence,
}

impl LoweringError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        let message = match self {
            LoweringError::UnknownOccurrence(path) => format!(
                "solver lowering references occurrence '{}', which is not present in the \
                 resolved occurrence tree",
                path.leaf().local_name()
            ),
            LoweringError::NoRootOccurrence => {
                "solver lowering found no top-level (depth-1) occurrence to ground against"
                    .to_string()
            }
        };
        Diagnostic::new(
            DiagnosticCode::new("ASM", SeverityLetter::Error, 8)
                .expect("ASM-E008 is a valid diagnostic code"),
            Severity::Error,
            "assembly",
            "Solver lowering error",
            message,
        )
        .expect("severity matches code letter")
    }
}

/// [`lower`]'s result: the built [`AssemblyProblem`], the free-occurrence
/// list in variable order (`free_occurrences[i]` owns variables
/// `[i * 6, i * 6 + 6)`), and every mate/joint tag this baseline anchor
/// abstraction could not realize (`MateKind::Tangent` today -- see module
/// doc comment).
#[derive(Debug, Clone)]
pub struct Lowering {
    pub problem: AssemblyProblem,
    pub free_occurrences: Vec<OccurrencePath>,
    pub unsupported: Vec<String>,
}

/// Lowers a resolved occurrence tree plus its mate/joint relations into a
/// semantic-neutral [`AssemblyProblem`] (`AICAD-142`'s own numeric
/// vocabulary), applying this module's grounding/free-variable/anchor
/// policy. See module doc comment for the full policy.
pub fn lower(
    occurrences: &[Occurrence],
    mates: &[Mate],
    joints: &[Joint],
) -> Result<Lowering, LoweringError> {
    lower_with_free_variable_selection(
        occurrences,
        mates,
        joints,
        FreeVariableSelection::RelationTouched,
    )
}

/// Lowers using the "every non-ground occurrence is free" policy
/// `crate::dof`'s semantic DOF analysis (`AICAD-145`) needs, instead of
/// [`lower`]'s own solver-facing "only relation-touched occurrences become
/// free" narrowing. An occurrence no relation ever names still has a real,
/// meaningful 6 DOF as an unconstrained rigid body -- [`lower`]'s
/// narrowing is a solver variable-count optimization (fewer unknowns to
/// iterate), not a claim that such an occurrence has zero freedom, and
/// `crate::dof` must not confuse the two.
pub fn lower_for_dof_analysis(
    occurrences: &[Occurrence],
    mates: &[Mate],
    joints: &[Joint],
) -> Result<Lowering, LoweringError> {
    lower_with_free_variable_selection(
        occurrences,
        mates,
        joints,
        FreeVariableSelection::AllNonGround,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FreeVariableSelection {
    /// [`lower`]'s policy: only occurrences a mate/joint actually names.
    RelationTouched,
    /// [`lower_for_dof_analysis`]'s policy: every non-ground occurrence.
    AllNonGround,
}

fn lower_with_free_variable_selection(
    occurrences: &[Occurrence],
    mates: &[Mate],
    joints: &[Joint],
    selection: FreeVariableSelection,
) -> Result<Lowering, LoweringError> {
    let root_path = occurrences
        .iter()
        .find(|o| o.path().depth() == 1)
        .map(|o| o.path().clone())
        .ok_or(LoweringError::NoRootOccurrence)?;

    let world_by_path: BTreeMap<OccurrencePath, Transform> = occurrences
        .iter()
        .map(|o| (o.path().clone(), o.world_pose().transform()))
        .collect();

    // Every relation subject must be a known occurrence regardless of
    // which occurrences end up free -- this is a reference-integrity
    // check, not part of the free-variable policy itself.
    for mate in mates {
        for subject in mate.subjects() {
            check_known(&world_by_path, subject.occurrence())?;
        }
    }
    for joint in joints {
        for occurrence in [joint.parent(), joint.child()] {
            check_known(&world_by_path, occurrence.occurrence())?;
        }
    }

    let free_paths: BTreeSet<OccurrencePath> = match selection {
        FreeVariableSelection::RelationTouched => {
            let mut free_paths = BTreeSet::new();
            for mate in mates {
                for subject in mate.subjects() {
                    if *subject.occurrence() != root_path {
                        free_paths.insert(subject.occurrence().clone());
                    }
                }
            }
            for joint in joints {
                for occurrence in [joint.parent(), joint.child()] {
                    if *occurrence.occurrence() != root_path {
                        free_paths.insert(occurrence.occurrence().clone());
                    }
                }
            }
            free_paths
        }
        FreeVariableSelection::AllNonGround => world_by_path
            .keys()
            .filter(|path| **path != root_path)
            .cloned()
            .collect(),
    };
    let free_occurrences: Vec<OccurrencePath> = free_paths.into_iter().collect();
    let offset_by_path: BTreeMap<&OccurrencePath, usize> = free_occurrences
        .iter()
        .enumerate()
        .map(|(i, path)| (path, i * 6))
        .collect();

    let anchor_of = |path: &OccurrencePath| -> Anchor {
        let world = world_by_path[path];
        match offset_by_path.get(path) {
            Some(&offset) => Anchor::free(world, offset),
            None => Anchor::fixed(world),
        }
    };

    let variable_count = free_occurrences.len() * 6;
    let initial_guess = vec![0.0; variable_count];

    let mut residuals = Vec::new();
    let mut unsupported = Vec::new();
    for mate in mates {
        let [a_ref, b_ref] = mate.subjects();
        let a = anchor_of(a_ref.occurrence());
        let b = anchor_of(b_ref.occurrence());
        let tag = format!("mate:{}", mate.id().name());
        match mate_residuals(
            mate.kind(),
            a,
            b,
            mate.parameter().map(|p| p.magnitude),
            &tag,
        ) {
            Some(mut r) => residuals.append(&mut r),
            None => unsupported.push(tag),
        }
    }
    for joint in joints {
        let a = anchor_of(joint.parent().occurrence());
        let b = anchor_of(joint.child().occurrence());
        let tag = format!("joint:{}", joint.id().name());
        residuals.append(&mut joint_residuals(joint.kind(), a, b, &tag));
    }

    let problem = AssemblyProblem::new(variable_count, initial_guess, residuals)
        .map_err(problem_error_is_unreachable_here);

    Ok(Lowering {
        problem: problem.expect("lowering always builds a shape-valid AssemblyProblem"),
        free_occurrences,
        unsupported,
    })
}

fn check_known(
    world_by_path: &BTreeMap<OccurrencePath, Transform>,
    path: &OccurrencePath,
) -> Result<(), LoweringError> {
    if world_by_path.contains_key(path) {
        Ok(())
    } else {
        Err(LoweringError::UnknownOccurrence(path.clone()))
    }
}

/// `AssemblyProblem::new` can only fail on a length/finiteness mismatch,
/// neither of which `lower` can ever produce (variable_count always
/// matches `initial_guess.len()` by construction, and `0.0` is always
/// finite) -- this documents that unreachability instead of `.unwrap()`ing
/// silently.
fn problem_error_is_unreachable_here(err: ProblemError) -> ProblemError {
    unreachable!("lower() always builds a shape-valid AssemblyProblem, got {err:?}")
}

fn push_vec3(residuals: &mut Vec<Residual>, tag: &str, f: impl Fn(&[f64]) -> Vector3 + 'static) {
    let f = Rc::new(f);
    for (i, component) in ["x", "y", "z"].iter().enumerate() {
        let f = Rc::clone(&f);
        residuals.push(Residual::new(
            format!("{tag}.{component}"),
            Rc::new(move |vars: &[f64]| {
                let v = f(vars);
                match i {
                    0 => v.x,
                    1 => v.y,
                    _ => v.z,
                }
            }),
        ));
    }
}

fn push_scalar(residuals: &mut Vec<Residual>, tag: &str, f: impl Fn(&[f64]) -> f64 + 'static) {
    residuals.push(Residual::new(tag, Rc::new(f)));
}

fn point_diff(a: Anchor, b: Anchor) -> impl Fn(&[f64]) -> Vector3 {
    move |vars: &[f64]| b.point(vars) - a.point(vars)
}

fn axis_align(a: Anchor, b: Anchor) -> impl Fn(&[f64]) -> Vector3 {
    move |vars: &[f64]| {
        a.primary(vars)
            .as_vector3()
            .cross(b.primary(vars).as_vector3())
    }
}

/// The component of `b`'s origin offset from `a`'s origin that is
/// perpendicular to `a`'s primary axis -- zero exactly when `b`'s origin
/// lies on the line through `a`'s origin along `a`'s primary axis.
fn perp_offset(a: Anchor, b: Anchor) -> impl Fn(&[f64]) -> Vector3 {
    move |vars: &[f64]| {
        let dir = a.primary(vars).as_vector3();
        let diff = b.point(vars) - a.point(vars);
        diff - dir * diff.dot(dir)
    }
}

/// The component of `b`'s origin offset from `a`'s origin that is
/// parallel to `a`'s primary axis.
fn along_offset(a: Anchor, b: Anchor) -> impl Fn(&[f64]) -> f64 {
    move |vars: &[f64]| {
        let dir = a.primary(vars).as_vector3();
        (b.point(vars) - a.point(vars)).dot(dir)
    }
}

fn primary_diff(a: Anchor, b: Anchor) -> impl Fn(&[f64]) -> Vector3 {
    move |vars: &[f64]| b.primary(vars).as_vector3() - a.primary(vars).as_vector3()
}

fn secondary_diff(a: Anchor, b: Anchor) -> impl Fn(&[f64]) -> Vector3 {
    move |vars: &[f64]| b.secondary(vars).as_vector3() - a.secondary(vars).as_vector3()
}

/// Builds `mate`'s residual equations, or `None` if `kind` has no
/// realization at this anchor-frame granularity (see module doc comment
/// "Anchor geometry").
fn mate_residuals(
    kind: MateKind,
    a: Anchor,
    b: Anchor,
    parameter: Option<f64>,
    tag: &str,
) -> Option<Vec<Residual>> {
    let mut residuals = Vec::new();
    match kind {
        MateKind::Coincident => push_vec3(&mut residuals, tag, point_diff(a, b)),
        MateKind::Concentric => {
            push_vec3(&mut residuals, &format!("{tag}.axis"), axis_align(a, b));
            push_vec3(&mut residuals, &format!("{tag}.offset"), perp_offset(a, b));
        }
        MateKind::Parallel => push_vec3(&mut residuals, tag, axis_align(a, b)),
        MateKind::Perpendicular => push_scalar(&mut residuals, tag, move |vars: &[f64]| {
            a.primary(vars).dot(b.primary(vars))
        }),
        MateKind::Tangent => return None,
        MateKind::Distance => {
            let target = parameter.expect("Mate::new requires Distance to bind a parameter");
            push_scalar(&mut residuals, tag, move |vars: &[f64]| {
                (b.point(vars) - a.point(vars)).length() - target
            });
        }
        MateKind::Angle => {
            let target = parameter.expect("Mate::new requires Angle to bind a parameter");
            push_scalar(&mut residuals, tag, move |vars: &[f64]| {
                a.primary(vars).dot(b.primary(vars)) - target.cos()
            });
        }
        MateKind::Lock => {
            push_vec3(&mut residuals, &format!("{tag}.point"), point_diff(a, b));
            push_vec3(
                &mut residuals,
                &format!("{tag}.primary"),
                primary_diff(a, b),
            );
            push_vec3(
                &mut residuals,
                &format!("{tag}.secondary"),
                secondary_diff(a, b),
            );
        }
    }
    Some(residuals)
}

/// Builds `joint`'s residual equations -- the DOF a family leaves free
/// (see `cad_assemblies::joint::JointKind::dof`) is exactly the
/// nullspace of the equations below; see module doc comment
/// "Representative-pose policy" for why the free DOF deterministically
/// keeps its authored value rather than drifting.
fn joint_residuals(kind: &JointKind, a: Anchor, b: Anchor, tag: &str) -> Vec<Residual> {
    let mut residuals = Vec::new();
    match kind {
        JointKind::Fixed => {
            push_vec3(&mut residuals, &format!("{tag}.point"), point_diff(a, b));
            push_vec3(
                &mut residuals,
                &format!("{tag}.primary"),
                primary_diff(a, b),
            );
            push_vec3(
                &mut residuals,
                &format!("{tag}.secondary"),
                secondary_diff(a, b),
            );
        }
        JointKind::Revolute { .. } => {
            push_vec3(&mut residuals, &format!("{tag}.point"), point_diff(a, b));
            push_vec3(&mut residuals, &format!("{tag}.axis"), axis_align(a, b));
        }
        JointKind::Prismatic { .. } => {
            push_vec3(
                &mut residuals,
                &format!("{tag}.primary"),
                primary_diff(a, b),
            );
            push_vec3(
                &mut residuals,
                &format!("{tag}.secondary"),
                secondary_diff(a, b),
            );
            push_vec3(&mut residuals, &format!("{tag}.offset"), perp_offset(a, b));
        }
        JointKind::Cylindrical { .. } => {
            push_vec3(&mut residuals, &format!("{tag}.axis"), axis_align(a, b));
            push_vec3(&mut residuals, &format!("{tag}.offset"), perp_offset(a, b));
        }
        JointKind::Planar { .. } => {
            push_vec3(&mut residuals, &format!("{tag}.normal"), primary_diff(a, b));
            push_scalar(&mut residuals, &format!("{tag}.along"), along_offset(a, b));
        }
    }
    residuals
}

/// [`AICAD-143`]'s own structural DOF evidence: the Jacobian rank of
/// `lowering.problem` at its own `initial_guess`, and the resulting
/// `remaining_dof = free_dof - rank`. This is *structural* evidence only
/// (no iteration, no convergence claim) -- exactly what `AICAD-143`'s
/// acceptance criterion needs before `AICAD-144`'s solver exists, and
/// what `AICAD-145`'s later semantic DOF analysis builds on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralDof {
    pub free_dof: usize,
    /// The Jacobian rank at whichever variable assignment was analyzed
    /// (the initial guess for [`structural_dof`]; an arbitrary point for
    /// [`structural_dof_at`]).
    pub rank: usize,
    pub remaining_dof: usize,
}

pub fn structural_dof(lowering: &Lowering, profile: &GroundingProfile) -> StructuralDof {
    structural_dof_at(&lowering.problem, lowering.problem.initial_guess(), profile)
}

/// [`structural_dof`], generalized to an arbitrary variable assignment --
/// `crate::baseline` reuses this at a *found solution* (not just the
/// initial guess) to classify whether a converged solve left any
/// direction genuinely free.
pub fn structural_dof_at(
    problem: &AssemblyProblem,
    at: &[f64],
    profile: &GroundingProfile,
) -> StructuralDof {
    let free_dof = problem.variable_count();
    let rank = if free_dof == 0 || problem.residuals().is_empty() {
        0
    } else {
        let jac = linalg::jacobian(
            |vars| problem.evaluate(vars),
            at,
            profile.finite_difference_step,
        );
        linalg::rank(&jac, profile.rank_tolerance)
    };
    StructuralDof {
        free_dof,
        rank,
        remaining_dof: free_dof - rank,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_assemblies::definition::ComponentDefinitionId;
    use cad_assemblies::frame::LocalPose;
    use cad_assemblies::instance::LogicalInstanceId;
    use cad_assemblies::joint::{JointAxis, JointId};
    use cad_assemblies::mate::MateId;
    use cad_assemblies::occurrence::OccurrencePath;
    use cad_assemblies::topology::OccurrenceTopologyRef;
    use cad_assemblies::value::ParameterValue;
    use cad_kernel_api::Vector3 as V3;
    use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn occurrence_path(local: &str) -> OccurrencePath {
        OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Part"),
            local,
        ))
    }

    fn occurrence(local: &str, translation: V3) -> Occurrence {
        occurrence_at_path(occurrence_path(local), translation)
    }

    fn occurrence_at_path(path: OccurrencePath, translation: V3) -> Occurrence {
        cad_assemblies::graph::expand(
            &{
                let mut registry = cad_assemblies::component::ComponentDefinitionRegistry::new();
                registry
                    .define(cad_assemblies::component::ComponentDefinition::new(
                        path.leaf().definition().clone(),
                        vec![],
                        vec![],
                    ))
                    .unwrap();
                registry
            },
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

    #[test]
    fn the_root_occurrence_never_becomes_a_free_variable() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 0.0, 0.0));
        let mate = Mate::new(
            MateId::named("m"),
            MateKind::Coincident,
            [face_subject(root.path()), face_subject(child.path())],
            None,
        )
        .unwrap();
        let lowering = lower(&[root.clone(), child.clone()], &[mate], &[]).unwrap();
        assert_eq!(lowering.free_occurrences, vec![child.path().clone()]);
        assert_eq!(lowering.problem.variable_count(), 6);
    }

    #[test]
    fn an_occurrence_touched_by_no_relation_never_becomes_a_free_variable() {
        let root = occurrence("root", V3::ZERO);
        let bystander = occurrence("bystander", V3::new(5.0, 0.0, 0.0));
        let lowering = lower(&[root, bystander], &[], &[]).unwrap();
        assert!(lowering.free_occurrences.is_empty());
        assert_eq!(lowering.problem.variable_count(), 0);
    }

    #[test]
    fn dof_analysis_lowering_frees_an_untouched_occurrence_that_lower_does_not() {
        let root = occurrence("root", V3::ZERO);
        let bystander = occurrence("bystander", V3::new(5.0, 0.0, 0.0));
        let for_solving = lower(&[root.clone(), bystander.clone()], &[], &[]).unwrap();
        let for_dof = lower_for_dof_analysis(&[root, bystander.clone()], &[], &[]).unwrap();
        assert!(for_solving.free_occurrences.is_empty());
        assert_eq!(for_dof.free_occurrences, vec![bystander.path().clone()]);
        assert_eq!(for_dof.problem.variable_count(), 6);
    }

    #[test]
    fn zero_variables_reproduce_the_authored_pose_exactly() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(2.0, 3.0, 4.0));
        let mate = Mate::new(
            MateId::named("m"),
            MateKind::Coincident,
            [face_subject(root.path()), face_subject(child.path())],
            None,
        )
        .unwrap();
        let lowering = lower(&[root, child.clone()], &[mate], &[]).unwrap();
        let residuals_at_zero = lowering.problem.evaluate(lowering.problem.initial_guess());
        // A Coincident mate between an occurrence at the origin and one
        // translated by (2,3,4) is NOT satisfied at the authored pose --
        // this only checks that the *authored* geometry is reproduced
        // (child's own world origin, at the all-zero delta) exactly.
        assert_eq!(residuals_at_zero, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn relation_order_does_not_change_the_free_variable_set() {
        let root = occurrence("root", V3::ZERO);
        let a = occurrence("a", V3::new(1.0, 0.0, 0.0));
        let b = occurrence("b", V3::new(0.0, 1.0, 0.0));
        let mate_1 = Mate::new(
            MateId::named("m1"),
            MateKind::Coincident,
            [face_subject(root.path()), face_subject(a.path())],
            None,
        )
        .unwrap();
        let mate_2 = Mate::new(
            MateId::named("m2"),
            MateKind::Coincident,
            [face_subject(a.path()), face_subject(b.path())],
            None,
        )
        .unwrap();
        let forward = lower(
            &[root.clone(), a.clone(), b.clone()],
            &[mate_1.clone(), mate_2.clone()],
            &[],
        )
        .unwrap();
        let reversed = lower(&[root, a, b], &[mate_2, mate_1], &[]).unwrap();
        assert_eq!(forward.free_occurrences, reversed.free_occurrences);
    }

    #[test]
    fn a_tangent_mate_is_reported_unsupported_not_silently_solved() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 0.0, 0.0));
        let mate = Mate::new(
            MateId::named("m"),
            MateKind::Tangent,
            [face_subject(root.path()), face_subject(child.path())],
            None,
        )
        .unwrap();
        let lowering = lower(&[root, child], &[mate], &[]).unwrap();
        assert_eq!(lowering.unsupported, vec!["mate:m".to_string()]);
        assert!(lowering.problem.residuals().is_empty());
    }

    #[test]
    fn an_unknown_occurrence_subject_fails_closed() {
        let root = occurrence("root", V3::ZERO);
        let ghost_path = occurrence_path("ghost");
        let mate = Mate::new(
            MateId::named("m"),
            MateKind::Coincident,
            [face_subject(root.path()), face_subject(&ghost_path)],
            None,
        )
        .unwrap();
        let err = lower(&[root], &[mate], &[]).unwrap_err();
        assert_eq!(err, LoweringError::UnknownOccurrence(ghost_path));
        assert_eq!(err.to_diagnostic().code.as_string(), "ASM-E008");
    }

    #[test]
    fn a_fully_locked_mate_removes_every_degree_of_freedom() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 2.0, 3.0));
        let mate = Mate::new(
            MateId::named("weld"),
            MateKind::Lock,
            [face_subject(root.path()), face_subject(child.path())],
            None,
        )
        .unwrap();
        let lowering = lower(&[root, child], &[mate], &[]).unwrap();
        let dof = structural_dof(&lowering, &GroundingProfile::v1());
        assert_eq!(dof.free_dof, 6);
        assert_eq!(dof.rank, 6);
        assert_eq!(dof.remaining_dof, 0);
    }

    #[test]
    fn a_revolute_joint_leaves_exactly_one_degree_of_freedom() {
        let root = occurrence("root", V3::ZERO);
        let arm = occurrence("arm", V3::new(1.0, 0.0, 0.0));
        let axis = JointAxis::unbounded(
            Dimension::Angle,
            ParameterValue::new(0.0, OperandType::dimensional(Dimension::Angle, None)),
        )
        .unwrap();
        let joint = Joint::new(
            JointId::named("hinge"),
            JointKind::revolute(axis).unwrap(),
            face_subject(root.path()),
            face_subject(arm.path()),
        );
        let lowering = lower(&[root, arm], &[], &[joint]).unwrap();
        let dof = structural_dof(&lowering, &GroundingProfile::v1());
        assert_eq!(dof.free_dof, 6);
        assert_eq!(dof.rank, 5);
        assert_eq!(dof.remaining_dof, 1);
    }

    #[test]
    fn an_untouched_occurrence_has_zero_free_dof() {
        let root = occurrence("root", V3::ZERO);
        let bystander = occurrence("bystander", V3::new(5.0, 0.0, 0.0));
        let lowering = lower(&[root, bystander], &[], &[]).unwrap();
        let dof = structural_dof(&lowering, &GroundingProfile::v1());
        assert_eq!(dof.free_dof, 0);
        assert_eq!(dof.remaining_dof, 0);
    }
}
